#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod erc20 {

    use ink_storage::collections::HashMap as StorageHashMap;

    #[ink(storage)]
    pub struct Erc20 {
        owner: AccountId,
        /// Total token supply
        total_supply: Balance,
        /// Mapping owner -> balance
        balances: StorageHashMap<AccountId, Balance>,
        allowances: StorageHashMap<(AccountId, AccountId), Balance>,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to:Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance,
    }

    #[ink(event)]
    pub struct Burn {
        #[ink(topic)]
        owner: AccountId,
        value: Balance,
    }

    /// Error types
    #[derive(Debug, PartialEq, Eq, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InsufficientBalance,
        InsufficientBurn,
        IssueNotOwner,
    }

    // Result type
    pub type Result<T> = core::result::Result<T, Error>;

    impl Erc20 {
        /// Creates a new ERC20 contract.
        #[ink(constructor)]
        pub fn new(init_supply: Balance) -> Self {
            let caller = Self::env().caller();
            let mut balances = StorageHashMap::new();
            balances.insert(caller, init_supply);

            let instance = Self {
                owner: caller,
                total_supply: init_supply,
                balances,
                allowances: StorageHashMap::new(),
            };

            // emit Transfer event
            Self::env().emit_event(Transfer {
                from: None,
                to: Some(caller),
                value: init_supply,
            });
            
            instance
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(&owner).copied().unwrap_or(0)
        }

        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowances.get(&(owner, spender)).copied().unwrap_or(0)
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let who = self.env().caller();
            self.transfer_helper(who, to, value)
        }

        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId,  to: AccountId, value: Balance) -> Result<()> {
            let caller = self.env().caller();
            let allowance = self.allowance(from, caller);
            if allowance < value {
                return Err(Error::InsufficientBalance);
            }

            self.allowances.insert((from, to), allowance - value);
            self.transfer_helper(from, to, value)
        }

        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            self.allowances.insert((owner, spender), value);

            self.env().emit_event(Approval {
                owner,
                spender,
                value,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn burn(&mut self, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            let balance = self.balance_of(owner);
            if balance < value {
                return Err(Error::InsufficientBurn);
            }

            self.balances.insert(owner, balance - value);
            self.total_supply = self.total_supply - value;

            Ok(())
        }

        #[ink(message)]
        pub fn issue(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let caller = self.env().caller();

            if caller != self.owner{
				return Err(Error::IssueNotOwner);
            }

            let to_balance = self.balance_of(to);

            self.balances.insert(to, to_balance + value);
            self.total_supply = self.total_supply + value;
            
            Ok(())
        }

        fn transfer_helper(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            let from_balance = self.balance_of(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(from, from_balance - value);
            let to_balance = self.balance_of(to);
            self.balances.insert(to, to_balance + value);

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value: value,
            });

            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;
        use ink_lang as ink;

        use ink_env::{
            hash::{
                Blake2x256,
                CryptoHash,
                HashOutput,
            },
            Clear,
        };

        type Event = <Erc20 as ::ink_lang::BaseEvent>::Type;

        fn assert_transfer_event(
            event: &ink_env::test::EmittedEvent,
            expected_from: Option<AccountId>,
            expected_to: Option<AccountId>,
            expected_value: Balance,
        ) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("encountered invalid contract event data buffer");
            if let Event::Transfer(Transfer { from, to, value }) = decoded_event {
                assert_eq!(from, expected_from, "encountered invalid Transfer.from");
                assert_eq!(to, expected_to, "encountered invalid Transfer.to");
                assert_eq!(value, expected_value, "encountered invalid Trasfer.value");
            } else {
                panic!("encountered unexpected event kind: expected a Transfer event")
            }
            fn encoded_into_hash<T>(entity: &T) -> Hash
            where
                T: scale::Encode,
            {
                let mut result = Hash::clear();
                let len_result = result.as_ref().len();
                let encoded = entity.encode();
                let len_encoded = encoded.len();
                if len_encoded <= len_result {
                    result.as_mut()[..len_encoded].copy_from_slice(&encoded);
                    return result
                }
                let mut hash_output =
                    <<Blake2x256 as HashOutput>::Type as Default>::default();
                <Blake2x256 as CryptoHash>::hash(&encoded, &mut hash_output);
                let copy_len = core::cmp::min(hash_output.len(), len_result);
                result.as_mut()[0..copy_len].copy_from_slice(&hash_output[0..copy_len]);
                result
            }
            let expected_topics = vec![
                encoded_into_hash(&PrefixedValue {
                    value: b"Erc20::Transfer",
                    prefix: b"",
                }),
                encoded_into_hash(&PrefixedValue {
                    prefix: b"Erc20::Transfer::from",
                    value: &expected_from,
                }),
                encoded_into_hash(&PrefixedValue {
                    prefix: b"Erc20::Transfer::to",
                    value: &expected_to,
                }),
                encoded_into_hash(&PrefixedValue {
                    prefix: b"Erc20::Transfer::value",
                    value: &expected_value,
                }),
            ];
            for (n, (actual_topic, expected_topic)) in
                event.topics.iter().zip(expected_topics).enumerate()
            {
                let topic = actual_topic
                    .decode::<Hash>()
                    .expect("encountered invalid topic encoding");
                assert_eq!(topic, expected_topic, "encountered invalid topic at {}", n);
            }
        }

        /// We test if the default constructor does its job.
        #[ink::test]
        fn create_contract_works() {
            // Constructor works.
            let erc20 = Erc20::new(1000);

            assert_eq!(erc20.total_supply(), 1000);
        }

        #[ink::test]
        fn balance_of_works() {
            let erc20 = Erc20::new(1000);

            let accounts = 
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                .expect("Cannot get accounts");

            assert_eq!(erc20.balance_of(accounts.alice), 1000);
            assert_eq!(erc20.balance_of(accounts.bob), 0);
        }

        #[ink::test]
        fn transfer_works() {
            // Constructor works.
            let mut erc20 = Erc20::new(1000);
            // Transfer event triggered during initial construction.
            let accounts =
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                    .expect("Cannot get accounts");

            assert_eq!(erc20.balance_of(accounts.bob), 0);
            // Alice transfers 10 tokens to Bob.
            assert_eq!(erc20.transfer(accounts.bob, 10), Ok(()));
            // Bob owns 10 tokens.
            assert_eq!(erc20.balance_of(accounts.bob), 10);

            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(emitted_events.len(), 2);

            assert_transfer_event(
                &emitted_events[0], 
                None, 
                Some(AccountId::from([0x01; 32])), 
                1000, 
            );

            // assert_transfer_event(
            //     &emitted_events[1],  
            //     Some(AccountId::from([0x01; 32])),
            //     Some(AccountId::from([0x02; 32])),
            //     10,
            // );
        }

        #[ink::test]
        fn burn_works() {
            // Constructor works.
            let mut erc20 = Erc20::new(100);
            // Transfer event triggered during initial construction.
            let accounts =
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                    .expect("Cannot get accounts");

            assert_eq!(erc20.burn(10), Ok(()));
            assert_eq!(erc20.balance_of(accounts.alice), 90);
        }
    }

        /// For calculating the event topic hash.
        struct PrefixedValue<'a, 'b, T> {
            pub prefix: &'a [u8],
            pub value: &'b T,
        }
    
        impl<X> scale::Encode for PrefixedValue<'_, '_, X>
        where
            X: scale::Encode,
        {
            #[inline]
            fn size_hint(&self) -> usize {
                self.prefix.size_hint() + self.value.size_hint()
            }
    
            #[inline]
            fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
                self.prefix.encode_to(dest);
                self.value.encode_to(dest);
            }
        }
}
