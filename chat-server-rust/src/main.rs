use actix::prelude::*;
use actix::{Actor, Addr, StreamHandler};
use actix_web::{
    web::{self, Data},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws;

const PORT: u16 = 4001;
const MAX_CLIENTS: usize = 32;

/// [MyWs] sends this to the ChatServer actor
#[derive(Message)]
#[rtype(result = "()")]
struct Connect {
    name: String,
    addr: Addr<MyWs>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Disconnect {
    addr: Addr<MyWs>,
}

/// Text-based messages sent between ChatServer and MyWs
#[derive(Message)]
#[rtype(result = "()")]
struct Message(String);

/// ChatServer actor, responsible for managing connected clients
/// - Keeps a list of connected clients
/// - Sends a 'ready' message to all connected clients to start the benchmark
/// - Broadcasts messages recieved to all connected clients
struct ChatServer {
    clients: Vec<Addr<MyWs>>,
}

impl ChatServer {
    fn new() -> Self {
        ChatServer {
            clients: Vec::new(),
        }
    }

    fn num_clients(&self) -> usize {
        self.clients.len()
    }

    /// Broadcast a message to all connected clients
    fn broadcast(&self, msg: &str) {
        for client in &self.clients {
            client.do_send(Message(msg.to_string()));
        }
    }

    /// Send a 'ready' message to all connected clients, starting the benchmark
    fn ready(&self) {
        self.broadcast("ready");
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

/// Handle [Message] messages from clients, broadcasting them to all connected clients
impl Handler<Message> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Message, _: &mut Self::Context) {
        self.broadcast(&msg.0);
    }
}

/// Handle [Connect] messages from clients, adding them to the list of connected clients
/// and counting the number of clients connected
impl Handler<Connect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Self::Context) {
        self.clients.push(msg.addr);
        println!(
            "{} connected ({} remain)",
            msg.name,
            MAX_CLIENTS - self.num_clients()
        );
        if self.num_clients() == MAX_CLIENTS {
            println!("All clients connected");
            println!("Starting benchmark by sending 'ready' message");
            self.ready();
        }
    }
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Self::Context) {
        self.clients.retain(|client| client != &msg.addr);
    }
}

/// WebSocket actor, responsible for handling messages with the client
struct MyWs {
    name: String,
    chat_server: Addr<ChatServer>,
}

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;

    /// Inform the [ChatServer] actor that a new client has [Connect]ed
    fn started(&mut self, ctx: &mut Self::Context) {
        self.chat_server.do_send(Connect {
            name: self.name.clone(),
            addr: ctx.address(),
        });
    }

    /// Inform the [ChatServer] actor that a client has [Disconnect]ed
    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.chat_server.do_send(Disconnect {
            addr: ctx.address(),
        });
    }
}

/// Handles incoming WebSocket messages
/// - Pings are responded to with a pong
/// - Text messages are sent to the [ChatServer] actor as a [Message]
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                self.chat_server
                    .do_send(Message(format!("{}: {}", self.name, text)));
            }
            _ => (),
        }
    }
}

/// Handler for [Message]s sent to the [MyWs] actor, which are then sent to the client
impl Handler<Message> for MyWs {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

#[actix_web::get("/")]
async fn handle_ws_connection(
    req: HttpRequest,
    chat_server: Data<Addr<ChatServer>>,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let name = req.query_string().rsplit_once("=").unwrap().1;
    ws::start(
        MyWs {
            name: name.to_string(),
            chat_server: chat_server.get_ref().clone(),
        },
        &req,
        stream,
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let chat_server = ChatServer::new().start();
    println!("Waiting for {} clients to connect...", MAX_CLIENTS);

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(chat_server.clone()))
            .service(handle_ws_connection)
    })
    .bind(("127.0.0.1", PORT))?
    .workers(1)
    .run()
    .await
}
