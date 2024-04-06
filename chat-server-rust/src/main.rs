use actix::prelude::*;
use actix::{Actor, Addr, StreamHandler};
use actix_web::{
    web::{self, Data},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws;

const PORT: u16 = 4001;
const MAX_CLIENTS: usize = 32;

#[derive(Message)]
#[rtype(result = "()")]
struct Connect {
    name: String,
    addr: Addr<MyWs>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Message(String);

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

    fn broadcast(&self, msg: &str) {
        for client in &self.clients {
            client.do_send(Message(msg.to_string()));
        }
    }

    fn ready(&self) {
        self.broadcast("ready");
    }
}

impl Handler<Message> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Message, _: &mut Self::Context) {
        self.broadcast(&msg.0);
    }
}

impl Handler<Message> for MyWs {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

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

/// Define HTTP actor
struct MyWs {
    name: String,
    chat_server: Addr<ChatServer>,
}

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.chat_server.do_send(Connect {
            name: self.name.clone(),
            addr: ctx.address(),
        });
    }
}

/// Handler for ws::Message message
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

async fn index(
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
            .route("/", web::get().to(index))
    })
    .bind(("127.0.0.1", PORT))?
    .workers(1)
    .run()
    .await
}
