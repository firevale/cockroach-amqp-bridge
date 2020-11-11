# Cockroach to AMQP bridge

#### Sync change data to RabbitMQ from CockroachDB

## Introduction

For traditional database, we have [Canal](https://github.com/alibaba/canal), [Maxwell](https://github.com/zendesk/maxwell) for mysql, or [pg-amqp-bridge](https://github.com/subzerocloud/pg-amqp-bridge) for Postgresql, to synchronize change data
from database to rabbitmq.

This tool achieves same feature via [experimental core changefeed](https://www.cockroachlabs.com/docs/v20.2/stream-data-out-of-cockroachdb-using-changefeeds.html#create-a-core-changefeed) for CockroachDB.

## Configuration

Configuration is done through environment variables:

- **DATABASE_URL**: e.g. `postgresql://username:password@domain.tld:port/database`
- **AMQP_URL**: e.g. `amqp://rabbitmq//`
- **AMQP_EXCHANGE**: rabbitmq exchange name to publish messages
- **TABLES**: e.g. `users,orders,goods`

## Running from source

#### Install Rust

```shell
curl https://sh.rustup.rs -sSf | sh
```

#### Run

```shell
DATABASE_URL="postgres://root@localhost:26257/db" \
TABLES="users,orders,goods" \
AMQP_URL="amqp://localhost//" \
AMQP_EXCHANGE="cockroach_change_feed" \
RUST_LOG=info \
cargo run
```

## Running as docker container

```shell
docker run --rm -it --net=host \
-e DATABASE_URL="postgres://postgres@localhost:26257/db" \
-e TABLES="users,orders,goods" \
-e AMQP_URL="amqp://localhost//" \
-e AMQP_EXCHANGE="cockroach_change_feed" \
-e RUST_LOG=info \
firevale/cockroach-amqp-bridge
```

## Contributing

Anyone and everyone is welcome to contribute.

## Author

[@xbinxu](https://github.com/xbinxu)

## License

Copyright © 2013-Present 北京火谷网络科技股份有限公司<br />
This source code is licensed under [MIT](https://github.com/subzerocloud/pg-amqp-bridge/blob/master/LICENSE.txt) license<br />
