// Magical Bitcoin Library
// Written in 2020 by
//     Alekos Filini <alekos.filini@gmail.com>
//
// Copyright (c) 2020 Magical Bitcoin
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::collections::BTreeMap;
use std::default::Default;

use bitcoin::{Address, OutPoint, SigHashType, Transaction};

use super::coin_selection::{CoinSelectionAlgorithm, DefaultCoinSelectionAlgorithm};
use crate::types::{FeeRate, UTXO};

#[derive(Debug, Default)]
pub struct TxBuilder<Cs: CoinSelectionAlgorithm> {
    pub(crate) recipients: Vec<(Address, u64)>,
    pub(crate) send_all: bool,
    pub(crate) fee_rate: Option<FeeRate>,
    pub(crate) policy_path: Option<BTreeMap<String, Vec<usize>>>,
    pub(crate) utxos: Option<Vec<OutPoint>>,
    pub(crate) unspendable: Option<Vec<OutPoint>>,
    pub(crate) sighash: Option<SigHashType>,
    pub(crate) ordering: TxOrdering,
    pub(crate) locktime: Option<u32>,
    pub(crate) rbf: Option<u32>,
    pub(crate) version: Option<Version>,
    pub(crate) change_policy: ChangeSpendPolicy,
    pub(crate) force_non_witness_utxo: bool,
    pub(crate) coin_selection: Cs,
}

impl TxBuilder<DefaultCoinSelectionAlgorithm> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_recipients(recipients: Vec<(Address, u64)>) -> Self {
        Self::default().set_recipients(recipients)
    }
}

impl<Cs: CoinSelectionAlgorithm> TxBuilder<Cs> {
    pub fn set_recipients(mut self, recipients: Vec<(Address, u64)>) -> Self {
        self.recipients = recipients;
        self
    }

    pub fn add_recipient(mut self, address: Address, amount: u64) -> Self {
        self.recipients.push((address, amount));
        self
    }

    pub fn send_all(mut self) -> Self {
        self.send_all = true;
        self
    }

    pub fn fee_rate(mut self, fee_rate: FeeRate) -> Self {
        self.fee_rate = Some(fee_rate);
        self
    }

    pub fn policy_path(mut self, policy_path: BTreeMap<String, Vec<usize>>) -> Self {
        self.policy_path = Some(policy_path);
        self
    }

    /// These have priority over the "unspendable" utxos
    pub fn utxos(mut self, utxos: Vec<OutPoint>) -> Self {
        self.utxos = Some(utxos);
        self
    }

    /// This has priority over the "unspendable" utxos
    pub fn add_utxo(mut self, utxo: OutPoint) -> Self {
        self.utxos.get_or_insert(vec![]).push(utxo);
        self
    }

    pub fn unspendable(mut self, unspendable: Vec<OutPoint>) -> Self {
        self.unspendable = Some(unspendable);
        self
    }

    pub fn add_unspendable(mut self, unspendable: OutPoint) -> Self {
        self.unspendable.get_or_insert(vec![]).push(unspendable);
        self
    }

    pub fn sighash(mut self, sighash: SigHashType) -> Self {
        self.sighash = Some(sighash);
        self
    }

    pub fn ordering(mut self, ordering: TxOrdering) -> Self {
        self.ordering = ordering;
        self
    }

    pub fn nlocktime(mut self, locktime: u32) -> Self {
        self.locktime = Some(locktime);
        self
    }

    pub fn enable_rbf(self) -> Self {
        self.enable_rbf_with_sequence(0xFFFFFFFD)
    }

    pub fn enable_rbf_with_sequence(mut self, nsequence: u32) -> Self {
        self.rbf = Some(nsequence);
        self
    }

    pub fn version(mut self, version: u32) -> Self {
        self.version = Some(Version(version));
        self
    }

    pub fn do_not_spend_change(mut self) -> Self {
        self.change_policy = ChangeSpendPolicy::ChangeForbidden;
        self
    }

    pub fn only_spend_change(mut self) -> Self {
        self.change_policy = ChangeSpendPolicy::OnlyChange;
        self
    }

    pub fn change_policy(mut self, change_policy: ChangeSpendPolicy) -> Self {
        self.change_policy = change_policy;
        self
    }

    pub fn force_non_witness_utxo(mut self) -> Self {
        self.force_non_witness_utxo = true;
        self
    }

    pub fn coin_selection<P: CoinSelectionAlgorithm>(self, coin_selection: P) -> TxBuilder<P> {
        TxBuilder {
            recipients: self.recipients,
            send_all: self.send_all,
            fee_rate: self.fee_rate,
            policy_path: self.policy_path,
            utxos: self.utxos,
            unspendable: self.unspendable,
            sighash: self.sighash,
            ordering: self.ordering,
            locktime: self.locktime,
            rbf: self.rbf,
            version: self.version,
            change_policy: self.change_policy,
            force_non_witness_utxo: self.force_non_witness_utxo,
            coin_selection,
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy)]
pub enum TxOrdering {
    Shuffle,
    Untouched,
    BIP69Lexicographic,
}

impl Default for TxOrdering {
    fn default() -> Self {
        TxOrdering::Shuffle
    }
}

impl TxOrdering {
    pub fn sort_tx(&self, tx: &mut Transaction) {
        match self {
            TxOrdering::Untouched => {}
            TxOrdering::Shuffle => {
                use rand::seq::SliceRandom;
                #[cfg(test)]
                use rand::SeedableRng;

                #[cfg(not(test))]
                let mut rng = rand::thread_rng();
                #[cfg(test)]
                let mut rng = rand::rngs::StdRng::seed_from_u64(0);

                tx.output.shuffle(&mut rng);
            }
            TxOrdering::BIP69Lexicographic => {
                tx.input.sort_unstable_by_key(|txin| {
                    (txin.previous_output.txid, txin.previous_output.vout)
                });
                tx.output
                    .sort_unstable_by_key(|txout| (txout.value, txout.script_pubkey.clone()));
            }
        }
    }
}

// Helper type that wraps u32 and has a default value of 1
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy)]
pub(crate) struct Version(pub(crate) u32);

impl Default for Version {
    fn default() -> Self {
        Version(1)
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy)]
pub enum ChangeSpendPolicy {
    ChangeAllowed,
    OnlyChange,
    ChangeForbidden,
}

impl Default for ChangeSpendPolicy {
    fn default() -> Self {
        ChangeSpendPolicy::ChangeAllowed
    }
}

impl ChangeSpendPolicy {
    pub(crate) fn filter_utxos<I: Iterator<Item = UTXO>>(&self, iter: I) -> Vec<UTXO> {
        match self {
            ChangeSpendPolicy::ChangeAllowed => iter.collect(),
            ChangeSpendPolicy::OnlyChange => iter.filter(|utxo| utxo.is_internal).collect(),
            ChangeSpendPolicy::ChangeForbidden => iter.filter(|utxo| !utxo.is_internal).collect(),
        }
    }
}

#[cfg(test)]
mod test {
    const ORDERING_TEST_TX: &'static str = "0200000003c26f3eb7932f7acddc5ddd26602b77e7516079b03090a16e2c2f54\
                                            85d1fd600f0100000000ffffffffc26f3eb7932f7acddc5ddd26602b77e75160\
                                            79b03090a16e2c2f5485d1fd600f0000000000ffffffff571fb3e02278217852\
                                            dd5d299947e2b7354a639adc32ec1fa7b82cfb5dec530e0500000000ffffffff\
                                            03e80300000000000002aaeee80300000000000001aa200300000000000001ff\
                                            00000000";
    macro_rules! ordering_test_tx {
        () => {
            deserialize::<bitcoin::Transaction>(&Vec::<u8>::from_hex(ORDERING_TEST_TX).unwrap())
                .unwrap()
        };
    }

    use bitcoin::consensus::deserialize;
    use bitcoin::hashes::hex::FromHex;

    use super::*;

    #[test]
    fn test_output_ordering_default_shuffle() {
        assert_eq!(TxOrdering::default(), TxOrdering::Shuffle);
    }

    #[test]
    fn test_output_ordering_untouched() {
        let original_tx = ordering_test_tx!();
        let mut tx = original_tx.clone();

        TxOrdering::Untouched.sort_tx(&mut tx);

        assert_eq!(original_tx, tx);
    }

    #[test]
    fn test_output_ordering_shuffle() {
        let original_tx = ordering_test_tx!();
        let mut tx = original_tx.clone();

        TxOrdering::Shuffle.sort_tx(&mut tx);

        assert_eq!(original_tx.input, tx.input);
        assert_ne!(original_tx.output, tx.output);
    }

    #[test]
    fn test_output_ordering_bip69() {
        use std::str::FromStr;

        let original_tx = ordering_test_tx!();
        let mut tx = original_tx.clone();

        TxOrdering::BIP69Lexicographic.sort_tx(&mut tx);

        assert_eq!(
            tx.input[0].previous_output,
            bitcoin::OutPoint::from_str(
                "0e53ec5dfb2cb8a71fec32dc9a634a35b7e24799295ddd5278217822e0b31f57:5"
            )
            .unwrap()
        );
        assert_eq!(
            tx.input[1].previous_output,
            bitcoin::OutPoint::from_str(
                "0f60fdd185542f2c6ea19030b0796051e7772b6026dd5ddccd7a2f93b73e6fc2:0"
            )
            .unwrap()
        );
        assert_eq!(
            tx.input[2].previous_output,
            bitcoin::OutPoint::from_str(
                "0f60fdd185542f2c6ea19030b0796051e7772b6026dd5ddccd7a2f93b73e6fc2:1"
            )
            .unwrap()
        );

        assert_eq!(tx.output[0].value, 800);
        assert_eq!(tx.output[1].script_pubkey, From::from(vec![0xAA]));
        assert_eq!(tx.output[2].script_pubkey, From::from(vec![0xAA, 0xEE]));
    }

    fn get_test_utxos() -> Vec<UTXO> {
        vec![
            UTXO {
                outpoint: OutPoint {
                    txid: Default::default(),
                    vout: 0,
                },
                txout: Default::default(),
                is_internal: false,
            },
            UTXO {
                outpoint: OutPoint {
                    txid: Default::default(),
                    vout: 1,
                },
                txout: Default::default(),
                is_internal: true,
            },
        ]
    }

    #[test]
    fn test_change_spend_policy_default() {
        let change_spend_policy = ChangeSpendPolicy::default();
        let filtered = change_spend_policy.filter_utxos(get_test_utxos().into_iter());

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_change_spend_policy_no_internal() {
        let change_spend_policy = ChangeSpendPolicy::ChangeForbidden;
        let filtered = change_spend_policy.filter_utxos(get_test_utxos().into_iter());

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].is_internal, false);
    }

    #[test]
    fn test_change_spend_policy_only_internal() {
        let change_spend_policy = ChangeSpendPolicy::OnlyChange;
        let filtered = change_spend_policy.filter_utxos(get_test_utxos().into_iter());

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].is_internal, true);
    }

    #[test]
    fn test_default_tx_version_1() {
        let version = Version::default();
        assert_eq!(version.0, 1);
    }
}
