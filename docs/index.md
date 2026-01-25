---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
  name: "NodeGet"
  text: 下一代服务器监控管理工具
  tagline: Next-generation server monitoring and management tools
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/quick-start
    - theme: alt
      text: API文档
      link: /api/reference
  image:
    src: /logo.png
    alt: NodeGet

features:
  - title: 细粒度权限
    details: 支持基于token的细粒度权限模型，大大增强安全性和灵活性，方便与其三方系统对接
  - title: 多服务器分布式
    details: 一个agent可以对接多个server，server间可以相互对接
  - title: 前后端分离
    details: 完全前后端分离，headless模式允许前端开发者自由构建展示页面和控制面板
---

<style>
:root {
    --vp-home-hero-name-color: transparent;
    --vp-home-hero-name-background: linear-gradient(120deg, #b710b0, #fbc614);
    --vp-home-hero-image-background-image: linear-gradient(300deg, #b710af6c, #fbc5147d);
    --vp-home-hero-image-filter: blur(65px);
}
.main .heading .text {
  font-size:32px;
}
.main .tagline {
  font-size:22px;
  padding-top:0;
}
</style>