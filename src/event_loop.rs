use crate::{ClientResult, Command, ExecutionClient};
use barter::execution::FillEvent;
use std::collections::HashMap;
use futures::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::{Message as WsMessage};
use tracing::{info, warn};

pub type ClientOrderId = &'static str;

struct Runner<St, Input, Error, ExchangeMessage, Client>
where
    St: Stream<Item = Result<Input, Error>> + StreamExt + Unpin + Send,
    Client: ExecutionClient,
{
    stream: St,
    command_rx: mpsc::Receiver<Command>,
    orders: HashMap<ClientOrderId, oneshot::Sender<ClientResult<FillEvent>>>,
    client: Client,

}

impl<St, Input, Error, ExchangeMessage, Client> Runner<St, Input, Error, ExchangeMessage, Client>
where
    St: Stream<Item = Result<Input, Error>> + StreamExt + Unpin + Send,
    ExchangeMessage: DeserializeOwned,
    Client: ExecutionClient,
{
    async fn run(mut self) {
        loop {
            tokio::select! {

                input = self.stream.next() => {
                    // Deserialize Exchange message from next input Protocol message
                    let message = match ParseOutcome::from(input) {
                        ParseOutcome::Message(message) => message,
                        ParseOutcome::Continue => continue,
                        ParseOutcome::Break => break,
                    };

                    // Transform Exchange Message into Event

                    // Send back over oneshot
                }

                command = self.command_rx.recv() => if let Some(command) = command {
                    match command {
                        Command::OpenOrder((request, response_tx)) => {
                            // Generate ClientOrderId
                            let client_order_id = "client_order_id";

                            // Add response oneshot::Sender to state
                            self.orders.insert(client_order_id, response_tx);

                            // Action Request

                        }
                    }

                }

            }
        }
    }
}

pub enum ParseOutcome<ExchangeMessage> {
    Message(ExchangeMessage),
    Continue,
    Break
}

impl<Input, Error, ExchangeMessage> From<Option<Result<Input, Error>>> for ParseOutcome<ExchangeMessage> {
    fn from(next_message: Option<Result<Input, Error>>) -> Self {
        // Extract Protocol message
        let message = match next_message {
            Some(Ok(message)) => message,
            Some(Err(err)) => return ParseOutcome::Break,
            None => return ParseOutcome::Break,
        };

        // Map to ExchangeMessage
        ParseOutcome::from(message)
    }
}

impl<ExchangeMessage> From<WsMessage> for ParseOutcome<ExchangeMessage>
where
    ExchangeMessage: DeserializeOwned
{
    fn from(ws_message: WsMessage) -> Self {
        // Extract payload from WebSocket message
        let payload = match ws_message {
            WsMessage::Text(text_payload) => text_payload.as_bytes(),
            WsMessage::Binary(binary_payload) => binary_payload.as_bytes(),
            WsMessage::Close(closing_frame) => {
                info!(
                    payload = &*format!("{:?}", closing_frame),
                    "received WebSocket close message"
                );
                return ParseOutcome::Break
            },
            _ => return ParseOutcome::Continue
        };

        serde_json::from_slice::<ExchangeMessage>(payload)
            .map(ParseOutcome::<ExchangeMessage>::Message)
            .unwrap_or_else(|err| {
                warn!(
                    error = &*format!("{:?}", err),
                    payload = &*format!("{:?}", payload),
                    action = "skipping message",
                    "failed to deserialize WebSocket Message into domain specific Message"
                );
                ParseOutcome::Continue
            })
    }
}