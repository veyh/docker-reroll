FROM alpine:latest
RUN set -ex; \
  apk add curl zip; \
  curl -fsSL https://releases.hashicorp.com/http-echo/1.0.0/http-echo_1.0.0_linux_amd64.zip -o http-echo_1.0.0_linux_amd64.zip; \
  unzip http-echo_1.0.0_linux_amd64.zip; \
  rm http-echo_1.0.0_linux_amd64.zip; \
  install --mode +x http-echo /usr/bin;

CMD ["/usr/bin/http-echo", "-listen=:80", "-text=hello world"]
EXPOSE 80
