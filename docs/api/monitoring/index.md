# Monitoring 任务总览

Monitoring 是本项目的重要功能之一，也可以称为 `监控` / `Report` 等

## 上报结构体

在本项目中，有两种监控数据类型

- `StaticMonitoring`: 静态数据，一般不会改变
- `DynamicMonitoring`: 动态数据，根据系统实时变化

### StaticMonitoring

Static Monitoring 结构如下:

```json
{
  "uuid": "e8583352-39e8-5a5b-b66c-e450689088fd",
  "time": 1769341269012, // 毫秒时间戳
  "cpu": {
    "physical_cores": 16, // 物理核心数
    "logical_cores": 32, // 逻辑核心数
    "per_core": [ // 每一个核心的详细数据
      {
        "id": 1, // ID 从 1 开始
        "name": "CPU 1",
        "vendor_id": "AuthenticAMD",
        "brand": "AMD Ryzen 9 8945HX with Radeon Graphics"
      },
      {
        "id": 2,
        "name": "CPU 2",
        "vendor_id": "AuthenticAMD",
        "brand": "AMD Ryzen 9 8945HX with Radeon Graphics"
      }
      // 目前只列举 2 核
    ]
  },
  "system": { // 系统数据
    "system_name": "Windows", // 系统名称
    "system_kernel": "26200", // 内核版本
    "system_kernel_version": "Windows 11 IoT Enterprise LTSC 2024", // 长内核版本
    "system_os_version": "11 (26200)", // 系统版本
    "system_os_long_version": "Windows 11 IoT Enterprise LTSC 2024", // 长系统版本
    "distribution_id": "windows", // 发行版 ID
    "system_host_name": "DESKTOP-BI8T1T9", // 主机名
    "arch": "x86_64", // 架构
    "virtualization": "HyperV" // 虚拟化平台
  },
  "gpu": [ // GPU 数据，可有多个 GPU
    {
      "id": 1, // ID 从 1 开始 
      "name": "NVIDIA GeForce RTX 5060 Laptop GPU", // GPU 名称
      "cuda_cores": 3328, // Cuda 核心数 (NV ONLY)
      "architecture": "Blackwell" // GPU 架构
    }
    // 目前只列举一个 GPU
  ]
}
```

### DynamicMonitoring

Dynamic Monitoring 结构如下:

```json
{
  "uuid": "e8583352-39e8-5a5b-b66c-e450689088fd",
  "time": 1769344168646, // 毫秒时间戳
  "cpu": { // CPU 数据，使用率取值为 0~100，频率单位 MHz
    "per_core": [
      {
        "id": 1, // ID 从 1 开始
        "cpu_usage": 13.429359436035156,
        "frequency_mhz": 2007
      },
      {
        "id": 2,
        "cpu_usage": 1.8074264526367188,
        "frequency_mhz": 2007
      }
      // 目前只列举 2 核
    ],
    "total_cpu_usage": 4.038932800292969
  },
  "ram": { // Ram 数据，单位字节
    "total_memory": 68501925888,
    "available_memory": 41439596544, // 注意与下面 used_memory 区分
    "used_memory": 27062329344,
    "total_swap": 0,
    "used_swap": 0
  },
  "load": { // Load 数据，仅 Linux 或 MacOS 有效
    "one": 0,
    "five": 0,
    "fifteen": 0
  },
  "system": { // System 数据
    "boot_time": 1769337198, // 秒时间戳
    "uptime": 6970, // 从上次开机计算起的时间，单位秒
    "process_count": 313 // 进程数量
  },
  "disk": [ // Disk 数据
    {
      "kind": "Ssd", // 可选 Hdd / Ssd，注意大小写
      "name": "", // 硬盘名称
      "file_system": "NTFS", // 文件系统
      "mount_point": "C:\\", // 挂载点
      "total_space": 322057531392, // 单位字节，下同
      "available_space": 91563786240,
      "is_removable": false, // 可移动
      "is_read_only": false, // 只读
      "read_speed": 35741, // 单位字节每秒，下同
      "write_speed": 49550 
    },
    {
      "kind": "Hdd",
      "name": "RedmiBook遗产",
      "file_system": "NTFS",
      "mount_point": "E:\\",
      "total_space": 512109121536,
      "available_space": 446577369088,
      "is_removable": false,
      "is_read_only": false,
      "read_speed": 0,
      "write_speed": 6524466
    }
    // 目前只列举 2 个硬盘
  ],
  "network": { // Network 数据
    "interfaces": [ // 各网卡
      {
        "interface_name": "WLAN-Trdpkt Packet Driver (TRDPKT)-0000", // 网卡名
        "total_received": 527863209, // 单位字节，下同
        "total_transmitted": 484144450,
        "receive_speed": 5559, // 单位字节每秒，下同
        "transmit_speed": 1626
      },
      {
        "interface_name": "以太网 5",
        "total_received": 0,
        "total_transmitted": 0,
        "receive_speed": 0,
        "transmit_speed": 0
      }
      // 目前只列举 2 张网卡
    ],
    "udp_connections": 67, // UDP 连接数
    "tcp_connections": 165 // TCP 连接数
  },
  "gpu": [ // GPU 数据
    {
      "id": 1, // ID 从 1 开始
      "used_memory": 2169692160, // 单位字节，下同
      "total_memory": 8546942976,
      "graphics_clock_mhz": 510, // 单位 MHz，下同
      "sm_clock_mhz": 510,
      "memory_clock_mhz": 405,
      "video_clock_mhz": 622,
      "utilization_gpu": 5, // GPU 使用率
      "utilization_memory": 30, // Memory 读写频率 (非使用率)
      "temperature": 51 // 温度
    }
  ]
}
```

### 注意事项

在这两个结构体中，所有字段都是必要的，若没有请留空 (而不是不定义 / null)

目前没有对任何数据进行检测，特别是字符串字段，请保证上传的数据可以被公众展示、使用，勿携带隐私数据

多 CPU 核心、多 GPU 支持时，请确保 Static 与 Dynamic 数据中 ID 是对上的

由于各系统获取到的信息不尽相同，请尽力保证与官方 `nodeget-agent` 实现相同

## 查询获取结构体

调用者通过 `query` 方法获取到的数据结构有些不同于 StaticMonitoring / DynamicMonitoring

其中 `uuid` / `timestamp` 字段为必需，其他均为可选，是为了定向提供数据，解析时请注意

## 查询条件

### DataQueryField

查询需要使用到 `StaticDataQueryField` 或 `DynamicDataQueryField`，其可选值分别为:

- `StaticDataQueryField`: `cpu` / `system` / `gpu`
- `DynamicDataQueryField`: `cpu` / `ram` / `load` /  `system` / `disk` / `network` / `gpu`

### QueryCondition

不论是查询 Static 信息还是 Dynamic 信息，都需要用到统一的结构体 `QueryCondition`

其为 Rust Enum，解析时请注意

```rust
#[serde(rename_all = "snake_case")]
pub enum QueryCondition {
    Uuid(uuid::Uuid),
    TimestampFromTo(i64, i64), // start, end
    TimestampFrom(i64),        // start,
    TimestampTo(i64),          // end

    Limit(u64), // limit

    Last,
}
```

下面是一些解析的示例:

```json
{
    "uuid": "e8583352-39e8-5a5b-b66c-e450689088fd"
}

{
    "timestamp_from_to": [1769344168646, 1769344169646]
}

{
    "timestamp_from": 1769344168646
}

{
    "limit": 1000 // 依照 timestamp 最新的 1000 条
}

"last" // 对就是一个 `last`，无其他东西
```

#### 注意事项

`timestamp_from_to` 字段可看作是 `timestamp_from` 与 `timestamp_to` 的简略写法，下面的两种表达方式是等价的:

```json
{
    "timestamp_from_to": [1769344168646, 1769344169646]
}

[
    {
        "timestamp_from": 1769344168646
    },
    {
        "timestamp_to": 1769344169646
    }
]
```

`limit` 为 1 与 `last` 等价，在数据库层面限制查询结果，按照时间倒序排列

多个条件并存时，为 `AND`，即只查询满足所有条件的数据