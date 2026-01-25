export const themeZhConfig = {
    logo: '/logo.png',
    nav: [
        { text: '主页', link: '/' },
        { text: '快速上手', link: '/guide/quick-start' },
        { text: 'API参考', link: '/api/reference' },
    ],

    sidebar: {
        '/guide/':[
            {
                text: '安装',
                items: [
                    { text: '快速上手', link: '/guide/quick-start' },
                ]
            },
            {
                text: '配置',
                items: [
                    { text: 'nginx配置', link: '/guide/configuration/nginx' },
                    { text: 'server配置', link: '/guide/configuration/server' },
                    { text: 'agent配置', link: '/guide/configuration/agent' },
                    { text: 'cloudflare配置', link: '/guide/configuration/cloudflare' },
                ]
            }
        ],
        '/api/':[
            {
                text: '设计理念',
                items: [
                    { text: '架构概述', link: '/api/framework' },
                ]
            },
            {
                text: 'API文档',
                items: [
                    { text: 'API接口', link: '/api/reference' },
                ]
            },
        ]
    },
    socialLinks: [
        { icon: 'github', link: 'https://github.com/NodeSeekDev/NodeGet' }
    ]
}