FROM golang:1.23

WORKDIR /home/isucon/webapp/go

RUN apt-get update && apt-get install --no-install-recommends -y \
  default-mysql-client-core=1.1.0 \
 && apt-get clean \
 && rm -rf /var/lib/apt/lists/*

COPY . .
RUN go build -o webapp .

CMD ["/home/isucon/webapp/go/webapp"]
