# PDF2HTML Web

基于 [pdf2htmlEX](https://github.com/pdf2htmlEX/pdf2htmlEX) 的在线 PDF 转 HTML 服务，使用 Rust + Actix-web 构建。

![](https://img.996.ninja/ninjutsu/159842b857bd2d1185e08d3bd6c50edd.png)


## Live Demo

[https://pdf2html.gongju.dev/](https://pdf2html.gongju.dev/)

## 功能特性

- 🚀 在线 PDF 转 HTML，保持原始排版
- 📁 支持拖拽上传
- ⚙️ 丰富的转换选项（缩放、页面范围、嵌入选项等）
- 👀 转换后即时预览和下载
- 🧹 自动清理：转换后的文件 3 小时后自动删除
- 🐳 Docker 一键部署
- ☸️ 支持 Kubernetes 部署

## 快速开始

### Docker 运行

```bash
# 构建镜像
docker build -t pdf2html-web .

# 运行容器
docker run -d --name pdf2html-web -p 8080:8080 pdf2html-web
```

访问 http://localhost:8080

### Kubernetes 部署

```bash
# 创建 namespace
kubectl create namespace gongjudev

# 部署
kubectl apply -f k8s-deployment.yaml
```

## 转换选项

| 选项 | 说明 |
|------|------|
| zoom | 缩放比例 |
| fit_width | 适应宽度 (px) |
| fit_height | 适应高度 (px) |
| first_page | 起始页 |
| last_page | 结束页 |
| embed_css | 嵌入 CSS |
| embed_font | 嵌入字体 |
| embed_image | 嵌入图片 |
| embed_javascript | 嵌入 JavaScript |
| split_pages | 分页输出 |

## API

### POST /api/convert

上传 PDF 文件进行转换。

请求：`multipart/form-data`
- `file`: PDF 文件（必需）
- 其他转换选项（可选）

响应：
```json
{
  "success": true,
  "message": "Conversion successful",
  "html_url": "/output/{task_id}/{filename}.html",
  "filename": "{filename}.html"
}
```

## 项目结构

```
pdf2html-web/
├── Cargo.toml           # Rust 依赖配置
├── Dockerfile           # Docker 构建文件
├── k8s-deployment.yaml  # Kubernetes 部署配置
├── src/
│   └── main.rs          # Rust 后端代码
└── static/
    └── index.html       # 前端页面
```

## 技术栈

- 后端：Rust + Actix-web
- 前端：HTML + Bootstrap 5
- PDF 转换：pdf2htmlEX
- 容器化：Docker

## 许可证

本项目采用 [GPL-3.0](LICENSE) 许可证。

pdf2htmlEX 采用 GPL-3.0 许可证。
