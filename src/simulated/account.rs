use crate::{
    Open, Order,
    model::{AccountEvent, AccountEventKind, balance::Balance, trade::{SymbolFees, Trade, TradeId}},
    SymbolBalance
};
use barter_integration::model::{Instrument, Side, Symbol};
use barter_data::model::PublicTrade;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use num_traits::Zero;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct ClientAccount {
    pub fees_percent: f64,
    pub account_event_tx: mpsc::UnboundedSender<AccountEvent>,
    pub balances: HashMap<Symbol, Balance>,
    pub orders: HashMap<Instrument, ClientOrders>,
    pub trade_number: u64,
}

impl ClientAccount {
    pub fn orders(&self, instrument: &Instrument) -> &ClientOrders {
        self.orders
            .get(instrument)
            .unwrap_or_else(|| panic!("SimulatedExchange must be configured for Instrument: {instrument}"))
    }

    pub fn orders_mut(&mut self, instrument: &Instrument) -> &mut ClientOrders {
        self.orders
            .get_mut(instrument)
            .unwrap_or_else(|| panic!("SimulatedExchange must be configured for Instrument: {instrument}"))
    }

    pub fn balance(&self, symbol: &Symbol) -> &Balance {
        self.balances
            .get(symbol)
            .unwrap_or_else(|| panic!("SimulatedExchange must be configured for Symbol: {symbol}"))
    }

    pub fn balance_mut(&mut self, symbol: &Symbol) -> &mut Balance {
        self.balances
            .get_mut(symbol)
            .unwrap_or_else(|| panic!("SimulatedExchange must be configured for Symbol: {symbol}"))
    }

    pub fn match_orders(&mut self, instrument: Instrument, trade: PublicTrade) {
        if self.orders(&instrument).has_matching_bid(&trade) {
            self.match_bids(&instrument, &trade);
        }

        if self.orders(&instrument).has_matching_ask(&trade) {
            self.match_bids(&instrument, &trade);
        }
    }

    pub fn match_bids(&mut self, instrument: &Instrument, trade: &PublicTrade) {
        // Keep track of how much trade liquidity is remaining to match with
        let mut remaining_liquidity = trade.quantity;

        let best_bid = loop {
            // Pop the best bid Order<Open>
            let best_bid = match self.orders_mut(instrument).bids.pop() {
                Some(best_bid) => best_bid,
                None => break None,
            };

            // If current best bid is no longer a match, or the trade liquidity is exhausted
            if best_bid.state.price < trade.price || remaining_liquidity <= 0.0 {
                break Some(best_bid);
            }

            // Liquidity is either enough for a full-fill or partial-fill
            match Self::fill_kind(&best_bid, remaining_liquidity) {
                // Full Order<Open> fill
                OrderFill::Full => {
                    // Remove trade quantity from remaining liquidity
                    let trade_quantity = best_bid.state.remaining_quantity();
                    remaining_liquidity -= trade_quantity;

                    // Update balances & send AccountEvents to client
                    self.update_from_match(best_bid, trade_quantity);

                    // Exact full fill with zero remaining trade liquidity (highly unlikely)
                    if remaining_liquidity.is_zero() {
                        break None
                    }
                }

                // Partial Order<Open> fill with zero remaining trade liquidity
                OrderFill::Partial => {
                    // Partial-fill means trade quantity is all the remaining trade liquidity
                    let trade_quantity = remaining_liquidity;

                    // Update balances & send AccountEvents to client
                    self.update_from_match(best_bid.clone(), trade_quantity);

                    break Some(best_bid)
                }
            }
        };

        // If best bid had a partial fill or is no longer a match, push it back onto the end of bids
        if let Some(best_bid) = best_bid {
            self.orders_mut(instrument).bids.push(best_bid)
        }
    }

    pub fn fill_kind(order: &Order<Open>, liquidity: f64) -> OrderFill {
        match order.state.remaining_quantity() <= liquidity {
            true => OrderFill::Full,
            false => OrderFill::Partial
        }
    }

    pub fn update_from_match(&mut self, order: Order<Open>, trade_quantity: f64) {
        // Calculate the trade fees (denominated in base or quote depending on Order Side)
        let fees = self.calculate_fees(&order, trade_quantity);

        // Generate AccountEvent::Balances by updating the base & quote SymbolBalances
        let balance_event = self.update_balance_from_match(&order, trade_quantity, &fees);

        // Generate AccountEvent::Trade for the Order<Open> match
        let trade_event = AccountEvent {
            received_time: Utc::now(),
            exchange: order.exchange,
            kind: AccountEventKind::Trade(Trade {
                id: self.trade_id(),
                order_id: order.state.id,
                instrument: order.instrument,
                side: order.state.side,
                price: order.state.price,
                quantity: trade_quantity,
                fees
            })
        };

        // Send AccountEvents to client
        self.account_event_tx
            .send(trade_event)
            .expect("Client is offline - failed to send AccountEvent::Trade");
        self.account_event_tx
            .send(balance_event)
            .expect("Client is offline - failed to send AccountEvent::Balances");
    }

    pub fn calculate_fees(&self, order: &Order<Open>, trade_quantity: f64) -> SymbolFees {
        match order.state.side {
            Side::Buy => {
                SymbolFees::new(
                    order.instrument.base.clone(),
                    self.fees_percent * trade_quantity
                )
            }
            Side::Sell => {
                SymbolFees::new(
                    order.instrument.quote.clone(),
                    self.fees_percent * order.state.price * trade_quantity
                )
            }
        }
    }

    /// [`Order<Open>`] matches (trades) result in the [`Balance`] of the base & quote
    /// [`Symbol`] to change. The direction of each balance change will depend on if the matched
    /// [`Order<Open`] was a [`Side::Buy`] or [`Side::Sell`].
    ///
    /// A [`Side::Buy`] match causes the [`Symbol`] [`Balance`] of the base to increase by the
    /// `trade_quantity`, and the quote to decrease by the `trade_quantity * price`.
    ///
    /// A [`Side::Sell`] match causes the [`Symbol`] [`Balance`] of the base to decrease by the
    /// `trade_quantity`, and the quote to increase by the `trade_quantity * price`.
    pub fn update_balance_from_match(&mut self, order: &Order<Open>, trade_quantity: f64, fees: &SymbolFees) -> AccountEvent {
        // Todo: Turn this into template for Side::Buy balance update test case
        // 1. btc { total: 0, available: 0 }, usdt { total: 100, available: 100 }
        // 2. Open order to buy 1 btc for 100 usdt
        // '--> Fees are taken into account in the opened order, so new balance update is true
        // 3. btc { total: 0, available: 0 }, usdt { total: 100, available: 0 }
        // 4a. Partial Fill for 0.5 btc for 50 usdt
        // '--> fees taken out of bought 0.5 btc, eg/ fee_percent * 0.5
        // 5a. btc { total: 0.5, available: 0.5 }, usdt { total: 50, available: 0 }
        // '--> btc total & available = fee_percent * 0.5
        // 4b. Full Fill for 1 btc for 100 usdt
        // 5b. btc { total: 1.0, available: 1.0 }, usdt { total: 0.0, available: 0.0 }

        // Todo: Turn this into template for Side::Sell balance update test case
        // 1. btc { total: 1.0, available: 1.0 }, usdt { total: 0.0, available: 0.0 }
        // 2. Open order ot sell 1 btc for 100 usdt
        // 3. btc { total: 1.0, available: 0.0 }, usdt { total: 0.0, available: 0.0 }
        // 4a. Partial fill for 0.5 btc for 50 usdt
        // 5a. btc { total: 0.5, available: 0.0 }, usdt { total: 50.0, available: 50.0 }
        // '--> usdt total & available = (trade_quantity * price) - fees

        let Instrument { base, quote, ..} = &order.instrument;
        let mut new_base_balance = *self.balance(base);
        let mut new_quote_balance = *self.balance(quote);

        match order.state.side {
            Side::Buy => {
                // Base total & available increase by trade_quantity minus base fees
                let base_increase = trade_quantity - fees.fees;
                new_base_balance.total += base_increase;
                new_base_balance.available += base_increase;

                // Quote total decreases by (trade_quantity * price)
                // Note: available was already decreased by the opening of the Side::Buy order
                new_quote_balance.total -= trade_quantity * order.state.price;
            }

            Side::Sell => {
                // Base total decreases by trade_quantity
                // Note: available was already decreased by the opening of the Side::Sell order
                new_base_balance.total -= trade_quantity;

                // Quote total & available increase by (trade_quantity * price) minus quote fees
                let quote_increase = (trade_quantity * order.state.price) - fees.fees;
                new_quote_balance.total += quote_increase;
                new_quote_balance.available += quote_increase
            }
        }

        *self.balance_mut(base) = new_base_balance;

        *self.balance_mut(quote) = new_quote_balance;

        AccountEvent {
            received_time: Utc::now(),
            exchange: order.exchange.clone(),
            kind: AccountEventKind::Balances(vec![
                SymbolBalance::new(order.instrument.base.clone(), new_base_balance),
                SymbolBalance::new(order.instrument.quote.clone(), new_quote_balance),
            ])
        }
    }

    fn trade_id(&mut self) -> TradeId {
        self.trade_number += 1;
        TradeId(self.trade_number.to_string())
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct ClientOrders {
    pub bids: Vec<Order<Open>>,
    pub asks: Vec<Order<Open>>,
}

impl ClientOrders {
    pub fn has_matching_bid(&self, trade: &PublicTrade) -> bool {
        match self.bids.last() {
            Some(best_bid) if best_bid.state.price >= trade.price => true,
            _ => false,
        }
    }

    pub fn has_matching_ask(&self, trade: &PublicTrade) -> bool {
        match self.asks.last() {
            Some(best_ask) if best_ask.state.price <= trade.price => true,
            _ => false,
        }
    }

    pub fn num_orders(&self) -> usize {
        self.bids.len() + self.asks.len()
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum OrderFill {
    Full,
    Partial,
}

#[cfg(test)]
mod tests {
    use barter_integration::model::Side;
    use super::*;

    #[test]
    fn test_client_orders_has_matching_bid() {
        struct TestCase {
            client_orders: ClientOrders,
            input_trade: PublicTrade,
            expected: bool,
        }

        let tests = vec![
            TestCase { // TC0: No matching bids for trade since no bids
                client_orders: client_orders(vec![], vec![]),
                input_trade: trade(Side::Buy, 100.0, 1.0),
                expected: false,
            },
            TestCase { // TC1: No matching bids for trade
                client_orders: client_orders(
                    vec![open_order(Side::Buy, 50.0, 1.0, 0.0)],
                    vec![]
                ),
                input_trade: trade(Side::Buy, 100.0, 1.0),
                expected: false,
            },
            TestCase { // TC2: Exact matching bid for trade
                client_orders: client_orders(
                    vec![open_order(Side::Buy, 100.0, 1.0, 0.0)],
                    vec![]
                ),
                input_trade: trade(Side::Buy, 100.0, 1.0),
                expected: true,
            },
            TestCase { // TC3: Matching bid for trade
                client_orders: client_orders(
                    vec![open_order(Side::Buy, 150.0, 1.0, 0.0)],
                    vec![]
                ),
                input_trade: trade(Side::Buy, 100.0, 1.0),
                expected: true,
            },
            TestCase { // TC4: No matching bid for trade even though there is an intersecting ask
                client_orders: client_orders(
                    vec![],
                    vec![open_order(Side::Sell, 150.0, 1.0, 0.0)]
                ),
                input_trade: trade(Side::Buy, 100.0, 1.0),
                expected: false,
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = test.client_orders.has_matching_bid(&test.input_trade);
            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }

    #[test]
    fn test_client_orders_has_matching_ask() {
        struct TestCase {
            client_orders: ClientOrders,
            input_trade: PublicTrade,
            expected: bool,
        }

        let tests = vec![
            TestCase { // TC0: No matching ask for trade since no asks
                client_orders: client_orders(vec![], vec![]),
                input_trade: trade(Side::Sell, 100.0, 1.0),
                expected: false,
            },
            TestCase { // TC1: No matching ask for trade
                client_orders: client_orders(
                    vec![],
                    vec![open_order(Side::Sell, 150.0, 1.0, 0.0)],
                ),
                input_trade: trade(Side::Sell, 100.0, 1.0),
                expected: false,
            },
            TestCase { // TC2: Exact matching ask for trade
                client_orders: client_orders(
                    vec![],
                    vec![open_order(Side::Sell, 100.0, 1.0, 0.0)],
                ),
                input_trade: trade(Side::Sell, 100.0, 1.0),
                expected: true,
            },
            TestCase { // TC3: Matching ask for trade
                client_orders: client_orders(
                    vec![],
                    vec![open_order(Side::Sell, 150.0, 1.0, 0.0)],
                ),
                input_trade: trade(Side::Sell, 200.0, 1.0),
                expected: true,
            },
            TestCase { // TC4: No matching ask for trade even though there is an intersecting bid
                client_orders: client_orders(
                    vec![open_order(Side::Sell, 150.0, 1.0, 0.0)],
                    vec![],
                ),
                input_trade: trade(Side::Buy, 200.0, 1.0),
                expected: false,
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = test.client_orders.has_matching_ask(&test.input_trade);
            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }
}