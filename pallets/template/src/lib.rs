#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

use sp_core::crypto::KeyTypeId;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"love");
pub mod crypto {
    use super::KEY_TYPE;
    use sp_core::sr25519::Signature as Sr25519Signature;
    use sp_runtime::{
        app_crypto::{
			app_crypto, 
			sr25519,
		},
        traits::Verify,
        MultiSignature, 
		MultiSigner,
    };
    app_crypto!(sr25519, KEY_TYPE);

    pub struct OcwAuthId;

    impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for OcwAuthId {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }

    impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
        for OcwAuthId
        {
            type RuntimeAppPublic = Public;
            type GenericSignature = sp_core::sr25519::Signature;
            type GenericPublic = sp_core::sr25519::Public;
        }
}

#[frame_support::pallet]
pub mod pallet {
	use sp_runtime::{
		offchain::storage::{
			MutateStorageError,
			StorageRetrievalError, 
			StorageValueRef,
		},
		traits::Zero,
		offchain::{
			http, 
			Duration,
		},
        transaction_validity::{
            InvalidTransaction, 
            TransactionValidity, 
            ValidTransaction
        },
	};
	use serde::{
		Deserialize,
		Deserializer,
	};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_system::{
		offchain::{
			AppCrypto,
            CreateSignedTransaction, 
			SendSignedTransaction,
			Signer,
            SubmitTransaction,
		},
	};
	use sp_std::vec::Vec;
	use sp_std::vec;
    use core::{
        convert::TryInto, 
        fmt,
    };
    use sp_io::offchain_index;

    #[derive(Deserialize, Encode, Decode)]
    struct GithubInfo {
        #[serde(deserialize_with = "de_string_to_bytes")]
        login: Vec<u8>,
        #[serde(deserialize_with = "de_string_to_bytes")]
        blog: Vec<u8>,
        public_repos: u32,
    }

    pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
        where
        D: Deserializer<'de>,
        {
            let s: &str = Deserialize::deserialize(de)?;
            Ok(s.as_bytes().to_vec())
        }

    impl fmt::Debug for GithubInfo {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{{ login: {}, blog: {}, public_repos: {} }}",
                sp_std::str::from_utf8(&self.login).map_err(|_| fmt::Error)?,
                sp_std::str::from_utf8(&self.blog).map_err(|_| fmt::Error)?,
                &self.public_repos
                )
        }
    }

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config  + CreateSignedTransaction<Call<Self>> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
        #[pallet::constant]
		type UnsignedInterval: Get<Self::BlockNumber>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

    #[derive(Debug, Deserialize, Encode, Decode, Default)]
    struct IndexingData(Vec<u8>, u64);

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored(something, who));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => return Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}
		}

		#[pallet::weight(0)]
        pub fn offchain_submit_data(origin: OriginFor<T>, payload: Vec<u8>) -> DispatchResultWithPostInfo {
            let _who = ensure_signed(origin)?;
            log::info!("in offchain_submit_data call: {:?}", payload);
            Ok(().into())
        }

        #[pallet::weight(0)]
		pub fn offchain_submit_data_unsigned(origin: OriginFor<T>, n: T::BlockNumber) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
            log::info!("in offchain_submit_data_unsigned: {:?}", n);
            let block_number = frame_system::Pallet::<T>::block_number();
            log::info!("block_number: {:?}", &block_number);
			<NextUnsignedAt<T>>::put(block_number + T::UnsignedInterval::get());
			Ok(().into())
		}

        #[pallet::weight(100)]
        pub fn chain_save_local_storage(origin: OriginFor<T>, number: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            log::info!("chain_save_local_storage account: {:?}", who);

            let key = Self::derive_key(frame_system::Pallet::<T>::block_number());
            let data = IndexingData(b"chain_save_local_storage".to_vec(), number);
            offchain_index::set(&key, &data.encode());
            Ok(())
        }
	}

    #[pallet::storage]
    #[pallet::getter(fn next_unsigned_at)]
    pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn offchain_worker(block_number: T::BlockNumber) {
            log::info!("Hello World from offchain workers!: {:?}", block_number);

			// let timeout = sp_io::offchain::timestamp()
            //     .add(sp_runtime::offchain::Duration::from_millis(8_000));
            // sp_io::offchain::sleep_until(timeout);

			if block_number % 2u32.into() != Zero::zero() {
                let key = Self::derive_key(block_number);
                let val_ref = StorageValueRef::persistent(&key);
                //  get a local random value 
                let random_slice = sp_io::offchain::random_seed();
                //  get a local timestamp
                let timestamp_u64 = sp_io::offchain::timestamp().unix_millis();
                // combine to a tuple and print it  
                let value = (random_slice, timestamp_u64);
                log::info!("in odd block, value to write: {:?}", value);

                //  set tuple content to key
                // val_ref.set(&value);

				struct StateError;
                //  mutate tuple content to key
                let res = val_ref.mutate(|val: Result<Option<([u8;32], u64)>, StorageRetrievalError>| -> Result<_, StateError> {
                    match val {
                        Ok(Some(_)) => Ok(value),
                        _ => Ok(value),
                    }
                });
                match res {
                    Ok(value) => {
                        log::info!("in odd block, mutate successfully: {:?}", value);
                    },
                    Err(MutateStorageError::ValueFunctionFailed(_)) => (),
                    Err(MutateStorageError::ConcurrentModification(_)) => (),
                }
            } else {
                let key = Self::derive_key(block_number - 1u32.into());
                let mut val_ref = StorageValueRef::persistent(&key);
                // get from db by key
                if let Ok(Some(value)) = val_ref.get::<([u8;32], u64)>() {
                    // print values
                    log::info!("in even block, value read: {:?}", value);
                    // delete that key
                    val_ref.clear();
                }
            }

			if let Ok(info) = Self::fetch_github_info() {
                log::info!("Github Info: {:?}", info);
            } else {
                log::info!("Error while fetch github info!");
            }

			let payload: Vec<u8> = vec![1,2,3,4,5,6,7,8];
            _ = Self::send_signed_tx(payload);

            let next_unsigned_at = <NextUnsignedAt<T>>::get();
            log::info!("block state: {:?}, {:?}",next_unsigned_at, block_number);
            if next_unsigned_at > block_number {
                log::error!("Too early to send unsigned transaction");
            }else{
                let value = block_number;
                let call = Call::offchain_submit_data_unsigned { n: value };
                let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
                    .map_err(|_| { log::error!("Failed in offchain_unsigned_tx"); });
            }
            
            let key = Self::derive_key(block_number);
            let val_ref = StorageValueRef::persistent(&key);
            if let Ok(Some(data)) = val_ref.get::<IndexingData>() {
                log::info!("local storage data: {:?}, {:?}",data.0, data.1);
            } else {
                log::error!("Error reading from local storage.");
            }
            
            log::info!("Leave from offchain workers!: {:?}", block_number);
        }

		// fn on_initialize(_n: T::BlockNumber) -> Weight {
        //     log::info!("in on_initialize!");
        //     0
        // }

        // fn on_finalize(_n: T::BlockNumber) {
        //     log::info!("in on_finalize!");
        // }

        // fn on_idle(_n: T::BlockNumber, _remaining_weight: Weight) -> Weight {
        //     log::info!("in on_idle!");
        //     0
        // }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            if let Call::offchain_submit_data_unsigned { n: _ } = call {
                ValidTransaction::with_tag_prefix("OffchainWorker")
                    .priority(15_000)
                    .and_provides(1)
                    .longevity(3)
                    .propagate(true)
                    .build()
            } else {
                InvalidTransaction::Call.into()
            }
        }
    }

	impl<T: Config> Pallet<T> {
        #[deny(clippy::clone_double_ref)]
        fn derive_key(block_number: T::BlockNumber) -> Vec<u8> {
            block_number.using_encoded(|encoded_bn| {
                b"pallet::node-template"
                    .iter()
                    .chain(encoded_bn)
                    .copied()
                    .collect::<Vec<u8>>()
            })
        }

		fn fetch_github_info() -> Result<GithubInfo, http::Error> {
            // prepare for send request
            let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(8_000));
            let request =
                http::Request::get("https://api.github.com/orgs/substrate-developer-hub");
            let pending = request
                .add_header("User-Agent", "Substrate-Offchain-Worker")
                .deadline(deadline).send().map_err(|_| http::Error::IoError)?;
            let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
            if response.code != 200 {
                log::warn!("Unexpected status code: {}", response.code);
                return Err(http::Error::Unknown)
            }
            let body = response.body().collect::<Vec<u8>>();
            let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
                log::warn!("No UTF8 body");
                http::Error::Unknown
            })?;
            // parse the response str
            let gh_info: GithubInfo = 
                serde_json::from_str(body_str).map_err(|_| http::Error::Unknown)?;
            Ok(gh_info)
        }

		fn send_signed_tx(payload: Vec<u8>) -> Result<(), &'static str> {
            let signer = Signer::<T, T::AuthorityId>::all_accounts();
            if !signer.can_sign() {
                return Err(
                    "No local accounts available. Consider adding one via `author_insertKey` RPC.",
                    )
            }
            let results = signer.send_signed_transaction(|_account| {
                Call::offchain_submit_data { payload: payload.clone() }
            });
            for (acc, res) in &results {
                match res {
                    Ok(()) => log::info!("[{:?}] Submitted data:{:?}", acc.id, payload),
                    Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
                }
            }
            Ok(())
        }
    }
}
