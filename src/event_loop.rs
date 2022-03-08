// use crate::{Command, ExecutionClient};
// use barter::event::{Event, MessageTransmitter};
// use futures::{Stream, StreamExt};
// use futures::stream::Next;
// use serde::de::DeserializeOwned;
// use async_trait::async_trait;
// use tokio::sync::mpsc;
// use tokio_tungstenite::tungstenite::{Message as WsMessage};
// use tracing::{info, warn};
//
// pub type ClientOrderId = &'static str;
//
// #[async_trait]
// pub trait ExecutionHandler<St, Input, Error, ExchangeMessage>: Consumer<St> + Transformer<ExchangeMessage> + ExecutionClient + MessageTransmitter<Event> + MessageReceiver<Command>
// where
//     St: Stream<Item = Result<Input, Error>> + StreamExt + Unpin + Send,
//     ExchangeMessage: DeserializeOwned + Send,
//     ParseOutcome<ExchangeMessage>: From<Option<Result<Input, Error>>>
// {
//     async fn run(mut self)
//     where
//         Self: Sized
//     {
//         let mut command_rx = self.command_rx();
//
//         loop {
//             tokio::select! {
//                 // Consume next message from Protocol Stream
//                 next_message = self.next() => {
//                     // Deserialize Exchange message from next input Protocol message
//                     let message = match ParseOutcome::from(next_message) {
//                         ParseOutcome::Message(message) => message,
//                         ParseOutcome::Continue => continue,
//                         ParseOutcome::Break => break,
//                     };
//
//                     // Transform ExchangeMessage into Events
//                     let events = self.transform(message);
//
//                     // Send back over oneshot
//                     self.send_many(events.into_iter().collect())
//                 }
//
//                 // Receive Commands to action
//                 command = command_rx.recv() => if let Some(command) = command {
//                     match command {
//                         Command::OpenOrder((request, response_tx)) => {
//                             if let Err(err) = self.open_order(request).await {
//                                 response_tx.send(Err(err))
//                             }
//
//                         }
//                     }
//
//                 }
//             }
//         }
//     }
//
// }
//
// pub trait Consumer<St> {
//     fn next(&mut self) -> Next<St>;
// }
//
// pub trait Transformer<ExchangeMessage> {
//     type Events: IntoIterator<Item = Event>;
//     fn transform(&self, message: ExchangeMessage) -> Self::Events;
// }
//
// pub trait MessageReceiver<Message> {
//     fn command_rx(&mut self) -> mpsc::Receiver<Message>;
// }
//
// pub enum ParseOutcome<ExchangeMessage> {
//     Message(ExchangeMessage),
//     Continue,
//     Break
// }
//
// impl<Input, Error, ExchangeMessage> From<Option<Result<Input, Error>>> for ParseOutcome<ExchangeMessage>
// where
//     ExchangeMessage: DeserializeOwned,
//     ParseOutcome<ExchangeMessage>: From<Input>
//
// {
//     fn from(next_message: Option<Result<Input, Error>>) -> Self {
//         // Extract Protocol message
//         let message = match next_message {
//             Some(Ok(message)) => message,
//             Some(Err(_)) => return ParseOutcome::Break,
//             None => return ParseOutcome::Break,
//         };
//
//         // Map to ExchangeMessage
//         ParseOutcome::from(message)
//     }
// }
//
// impl<ExchangeMessage> From<WsMessage> for ParseOutcome<ExchangeMessage>
// where
//     ExchangeMessage: DeserializeOwned
// {
//     fn from(ws_message: WsMessage) -> Self {
//         // Extract payload from WebSocket message
//         let payload = match ws_message {
//             WsMessage::Text(text_payload) => text_payload.as_bytes().to_owned(),
//             WsMessage::Binary(binary_payload) => binary_payload,
//             WsMessage::Close(closing_frame) => {
//                 info!(
//                     payload = &*format!("{:?}", closing_frame),
//                     "received WebSocket close message"
//                 );
//                 return ParseOutcome::Break
//             },
//             _ => return ParseOutcome::Continue
//         };
//
//         serde_json::from_slice::<ExchangeMessage>(&payload)
//             .map(ParseOutcome::<ExchangeMessage>::Message)
//             .unwrap_or_else(|err| {
//                 warn!(
//                     error = &*format!("{:?}", err),
//                     payload = &*format!("{:?}", payload),
//                     action = "skipping message",
//                     "failed to deserialize WebSocket Message into domain specific Message"
//                 );
//                 ParseOutcome::Continue
//             })
//     }
// }