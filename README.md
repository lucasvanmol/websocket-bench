# websocket-server

This benchmarks a websocket server intended as a simple but very active chat room, inspired by [bun's own benchmarks against node and Deno](https://github.com/oven-sh/bun/blob/main/bench/websocket-server/). It aims to compare those benchmarks against Rust's [actix-web](https://actix.rs/docs/websockets/) framework.

First, start one of the servers. By default, it will wait for 32 clients which the client script will handle.

Run in Rust (`actix-web`):

```bash
cd chat-server-rust && cargo run --release
```

Run in Bun (`Bun.serve`):

```bash
bun ./chat-server.bun.js
```

Run in Node (`"ws"` package):

```bash
node ./chat-server.node.mjs
```

Then, run the client script. By default, it will connect 32 clients. This client script can run in Bun, Node, or Deno

```bash
node ./chat-client.mjs
```

The client script loops through a list of messages for each connected client and sends a message.

For example, when the client sends `"foo"`, the server sends back `"John: foo"` so that all members of the chatroom receive the message.

The client script waits until it receives all the messages for each client before sending the next batch of messages.