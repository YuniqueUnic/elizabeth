我希望前端和后端“一体化”, 从而就能解决下面的 Mixed Content 问题。
或者说还有什么简单且易用的解决方案？请你仔细思考并给出解决方案。

## deploy structure

        user
        ^
        |
    https://box.yunique.top(https://frpserver:443:http://frpserver:52033)
        ^
        |

---

        |
        nginx
        ^
        |
    reverse proxy(http://frpserver:52033 : http://frpclient:4001)
        ^
        |
    frp server (52033)
        ^
        |

---

        |
    frp client (4001)
        ^
        |
    docker bind (4001)
        ^
        |

^----------------^ | | frontend backend (4001:4001) (-:4092)

## 访问流程

1. **用户访问**：用户访问 https://box.yunique.top，nginx 将请求转发到 frp server
   的 52033 端口。
2. **frp server**：frp server 将请求转发到 frp client 的 4001 端口。
3. **frp client**：frp client 将请求转发到 docker 容器的 4001 端口。
4. **docker 容器**：docker 容器将请求转发到 4001 端口。
5. docker 容器内有：frontend(4001), backend(4092)
6. frontend 调用 backend api, 从 frontend:4001 访问 backend:4092.

## 当前的问题

在上述的访问流程中，用户端是通过 https 来发起的请求，而实际内部是 http
的。以至于有 浏览器的 mixed-content
请求。浏览器会拒绝这些请求，导致用户无法访问到后端服务。Mixed Content: The page
at 'https://box.yunique.top/' was loaded over HTTPS, but requested an insecure
resource 'http://elizabeth-backend:4092/api/v1/rooms/hello?password=abc123'.
This request has been blocked; the content must be served over HTTPS.

## 相关文件

@docker-compose.yml the frontend and backend in the same docker-compose bu not
the same docker container. @nginx_elizabeth.conf this the frontend nginx
configuration. @niginx.conf this is the nginx main configuration.

And if you encounter any issues, please use the web search or related tools to
help you solve this problem correctly.
