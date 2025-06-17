use dioxus::prelude::*;
use server_fn::codec::{StreamingText, TextStream};
use futures::StreamExt;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // todo
    let mut message: Signal<String> = use_signal(|| String::new());

    let mut messages: Signal<Vec<Message>> = use_signal(|| Vec::<Message>::new());

    use_effect(move || {

        use_future(move || async move {
            for message in get_message().await.unwrap(){
                messages.write().push(message);
            }
        }) ;

        spawn(
            async move {

                let stream: TextStream = message_stream().await.unwrap();
                let mut stream = stream.into_inner();

                while let Some(Ok(message)) = stream.next().await {
                    let message : Message = serde_json::from_str(&message).unwrap();
                    
                    messages.write().push(message);
                }
            }
        );
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") },
        div {
            class : "main-container",
            div {
                class : "message-header",
                "Messages"
            }, 
            div {
                class : "message-list",
                for message in messages.iter() {
                    div {
                        class : "message-row",
                        div {
                            class : "message-item",
                            div { { message.message.clone() } } 
                        }
                    }
                }
            },
            footer {
                div {
                    class : "message-input-container",
                    textarea { 
                        oninput : move |e| {
                            message.set(e.value())
                        }
                    },
                    button {
                        class : "btn-send",
                        onclick : move |_| async move {
                            send_message(message()).await.unwrap();
                        },
                        "SEND"
                    }
                }
            }
        }
    }
}

#[cfg(feature = "server")]
const DB_CON_URL : &'static str = "DB CON URL";

#[server(output = StreamingText)]
pub async fn message_stream() -> Result<TextStream, ServerFnError> {

    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {

        use mongodb::Client;

        let client = Client::with_uri_str(DB_CON_URL).await.unwrap();

        let db = client.database("chat");

        let col = db.collection::<Message>("messages");

        let mut stream = col.watch().await.unwrap();

        while let Some(change) = stream.next().await {
            if let Ok(event) = change {
                if let Some(full_doc) = event.full_document {

                    let json = serde_json::to_string(&full_doc).unwrap();

                    tx.unbounded_send(Ok(json)).unwrap();
                } 
            }
        }
    });

    Ok(TextStream::new(rx))
}

#[server]
pub async fn send_message(message : String) -> Result<(), ServerFnError> {

    use mongodb::Client;

    let client = Client::with_uri_str(DB_CON_URL).await.unwrap();

    let db = client.database("chat");

    db.collection::<Message>("messages").insert_one(Message {
        message
    })
    .await
    .unwrap();

    Ok(())
}

#[server]
pub async fn get_message() -> Result<Vec<Message>, ServerFnError> {

    use mongodb::Client;
    use mongodb::bson::doc;

    let client = Client::with_uri_str(DB_CON_URL).await.unwrap();

    let db = client.database("chat");

    let mut cursor = db.collection::<Message>("messages").find(doc!{}).await.unwrap();

    let mut messages : Vec<Message> = Vec::new();

    while let Some(result) = cursor.next().await {
        if let Ok(message) = result {
            messages.push(message);
        }
    }

    Ok(messages)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Message {
    message : String
}
