//! GPU 监控数据采集模块。
//!
//! 通过 NVML（NVIDIA Management Library）获取 GPU 的静态信息（型号、CUDA 核心数、架构）
//! 与动态信息（显存、利用率、温度、时钟频率）。无 NVIDIA 驱动时返回空数据。

use crate::monitoring::get_global_gpu;
use ng_monitoring::data_structure::{DynamicGpuData, StaticGpuData};
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard, OnceCell};

/// 从 GPU 获取的静态数据结构，包含所有 GPU 的基本信息。
#[derive(Debug)]
pub struct StaticDataFromGpu(pub Vec<StaticGpuData>);

/// 全局静态 GPU 数据实例，用于缓存 GPU 静态信息。
static GLOBAL_STATIC_DATA_FROM_GPU: OnceCell<Mutex<StaticDataFromGpu>> = OnceCell::const_new();

impl StaticDataFromGpu {
    /// 创建新的静态 GPU 数据实例。
    ///
    /// 通过 NVML 获取 GPU 的静态信息，如设备 ID、名称、CUDA 核心数和架构。
    ///
    /// NVML 本身是阻塞式 FFI 调用；这里用 `block_in_place` 将当前 worker
    /// 标记为阻塞，允许 runtime 把其它 async 任务重新调度到兄弟 worker，
    /// 避免在占有 `get_global_gpu()` 互斥锁期间拖慢整个 runtime。
    ///
    /// 返回包含所有 GPU 静态数据的向量。
    pub async fn new() -> Self {
        let nvml_mutex = get_global_gpu().await;
        let nvml_guard = nvml_mutex.lock().await;

        let Some(nvml) = &*nvml_guard else {
            return Self(vec![]);
        };

        let data = tokio::task::block_in_place(|| {
            let gpu_count = nvml.device_count().unwrap_or(0);

            (0..gpu_count)
                .filter_map(|id| {
                    let device = nvml.device_by_index(id).ok()?;
                    // 字段级 fallback：任一可选字段失败不应让整张卡消失（见 #54）。
                    let name = device.name().unwrap_or_else(|_| format!("GPU {id}"));
                    let cuda_cores = u64::from(device.num_cores().unwrap_or(0));
                    let architecture = device
                        .architecture()
                        .map_or_else(|_| "unknown".to_owned(), |a| a.to_string());
                    Some(StaticGpuData {
                        id: id + 1,
                        name,
                        cuda_cores,
                        architecture,
                    })
                })
                .collect::<Vec<_>>()
        });

        Self(data)
    }

    /// 获取静态 GPU 数据的可变引用。
    ///
    /// 如果全局静态 GPU 数据实例不存在，则初始化它；否则直接返回现有的实例。
    ///
    /// 返回静态 GPU 数据的 `MutexGuard`。
    pub async fn get() -> MutexGuard<'static, Self> {
        let data_mutex = GLOBAL_STATIC_DATA_FROM_GPU
            .get_or_init(|| async { Mutex::new(Self::new().await) })
            .await;

        data_mutex.lock().await
    }
}

/// 从 GPU 获取的动态数据结构，包含所有 GPU 的实时性能数据。
#[derive(Debug)]
pub struct DynamicDataFromGpu(pub Arc<Vec<DynamicGpuData>>);

/// 全局动态 GPU 数据实例，用于缓存 GPU 动态信息。
static GLOBAL_DYNAMIC_DATA_FROM_GPU: OnceCell<Mutex<DynamicDataFromGpu>> = OnceCell::const_new();

impl DynamicDataFromGpu {
    /// 创建新的动态 GPU 数据实例。
    ///
    /// 通过 NVML 获取 GPU 的动态信息，如显存使用情况、利用率、温度和时钟频率。
    /// NVML 调用是阻塞式 FFI，这里使用 `block_in_place` 避免持锁期间卡住 runtime。
    ///
    /// 返回包含所有 GPU 动态数据的向量。
    async fn new() -> Self {
        let nvml_mutex = get_global_gpu().await;
        let data = {
            let nvml_guard = nvml_mutex.lock().await;

            let Some(nvml) = &*nvml_guard else {
                return Self(Arc::new(vec![]));
            };

            tokio::task::block_in_place(|| {
                let gpu_count = nvml.device_count().unwrap_or(0);

                (0..gpu_count)
                    .filter_map(|id| {
                        let device = nvml.device_by_index(id).ok()?;
                        let memory_usage = device.memory_info().ok()?;
                        let utilization = device.utilization_rates().ok()?;

                        Some(DynamicGpuData {
                            id: id + 1,
                            used_memory: memory_usage.used,
                            total_memory: memory_usage.total,
                            graphics_clock_mhz: device.clock_info(Clock::Graphics).ok()?.into(),
                            sm_clock_mhz: device.clock_info(Clock::SM).ok()?.into(),
                            memory_clock_mhz: device.clock_info(Clock::Memory).ok()?.into(),
                            video_clock_mhz: device.clock_info(Clock::Video).ok()?.into(),
                            utilization_gpu: utilization.gpu.try_into().ok()?,
                            utilization_memory: utilization.memory.try_into().ok()?,
                            temperature: device
                                .temperature(TemperatureSensor::Gpu)
                                .ok()?
                                .try_into()
                                .ok()?,
                        })
                    })
                    .collect::<Vec<_>>()
            })
        };

        Self(Arc::new(data))
    }

    /// 更新动态 GPU 数据。
    ///
    /// 刷新现有 GPU 数据，更新显存使用情况、利用率、温度和时钟频率等信息。
    ///
    /// 若 NVML 报告的设备数量超过当前缓存，额外的 GPU 会以默认值追加到尾部（#55 增量枚举），
    /// 以支持虚拟化场景下 vGPU 的热增。已存在的 GPU 条目不会重建，避免 FFI 反复读取静态字段。
    async fn update(&mut self) {
        let nvml_mutex = get_global_gpu().await;
        let nvml_guard = nvml_mutex.lock().await;

        let Some(nvml) = &*nvml_guard else { return };

        tokio::task::block_in_place(|| {
            let gpu_count = nvml.device_count().unwrap_or(0);
            let target = gpu_count as usize;

            // 从头构建 Vec，不依赖旧 Arc 数据。
            // 这避免了 Arc::unwrap_or_clone 在引用计数 > 1 时深拷贝整个 Vec
            // （上一次 tick 的 gpu_guard.0.clone() 可能延长 Arc 寿命）。
            // 与 CPU per_core 的重建策略一致；GPU 数据量极小（通常 1~8 张卡），
            // 每秒重建的开销可忽略。
            let mut data = Vec::with_capacity(target);

            for id in 0..target {
                let mut gpu_data = DynamicGpuData {
                    id: (id + 1) as u32,
                    used_memory: 0,
                    total_memory: 0,
                    graphics_clock_mhz: 0,
                    sm_clock_mhz: 0,
                    memory_clock_mhz: 0,
                    video_clock_mhz: 0,
                    utilization_gpu: 0,
                    utilization_memory: 0,
                    temperature: 0,
                };

                if let Ok(device) = nvml.device_by_index(id as u32) {
                    if let Ok(memory_usage) = device.memory_info() {
                        gpu_data.used_memory = memory_usage.used;
                        gpu_data.total_memory = memory_usage.total;
                    }

                    if let Ok(utilization) = device.utilization_rates() {
                        gpu_data.utilization_gpu = utilization.gpu as _;
                        gpu_data.utilization_memory = utilization.memory as _;
                    }

                    if let Ok(temp) = device.temperature(TemperatureSensor::Gpu) {
                        gpu_data.temperature = temp as _;
                    }

                    if let Ok(clock) = device.clock_info(Clock::Graphics) {
                        gpu_data.graphics_clock_mhz = clock.into();
                    }
                    if let Ok(clock) = device.clock_info(Clock::SM) {
                        gpu_data.sm_clock_mhz = clock.into();
                    }
                    if let Ok(clock) = device.clock_info(Clock::Memory) {
                        gpu_data.memory_clock_mhz = clock.into();
                    }
                    if let Ok(clock) = device.clock_info(Clock::Video) {
                        gpu_data.video_clock_mhz = clock.into();
                    }
                }

                data.push(gpu_data);
            }

            self.0 = Arc::new(data);
        });
    }

    /// 异步刷新并获取动态 GPU 数据。
    ///
    /// 如果全局动态 GPU 数据实例不存在，则初始化它；否则更新现有数据并返回。
    ///
    /// 返回动态 GPU 数据的 `MutexGuard`。
    pub async fn refresh_and_get() -> MutexGuard<'static, Self> {
        let data_mutex = GLOBAL_DYNAMIC_DATA_FROM_GPU
            .get_or_init(|| async { Mutex::new(Self::new().await) })
            .await;

        let mut data = data_mutex.lock().await;
        data.update().await;

        data
    }
}
