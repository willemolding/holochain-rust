use cas::content::{Address, AddressableContent, Content};
use eav::{EntityAttributeValue, EntityAttributeValueStorage};
use entry::{test_entry_unique, Entry};
use error::HolochainError;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::{mpsc::channel, Arc, RwLock},
    thread,
};

/// content addressable store (CAS)
/// implements storage in memory or persistently
/// anything implementing AddressableContent can be added and fetched by address
/// CAS is append only
pub trait ContentAddressableStorage: Clone + Send + Sync {
    /// adds AddressableContent to the ContentAddressableStorage by its Address as Content
    fn add(&mut self, content: &AddressableContent) -> Result<(), HolochainError>;
    /// true if the Address is in the Store, false otherwise.
    /// may be more efficient than retrieve depending on the implementation.
    fn contains(&self, address: &Address) -> Result<bool, HolochainError>;
    /// returns Some AddressableContent if it is in the Store, else None
    /// AddressableContent::from_content() can be used to allow the compiler to infer the type
    /// @see the fetch implementation for ExampleCas in the cas module tests
    fn fetch<C: AddressableContent>(&self, address: &Address) -> Result<Option<C>, HolochainError>;
}

#[derive(Clone)]
/// some struct to show an example ContentAddressableStorage implementation
/// this is a thread-safe wrapper around the non-thread-safe implementation below
/// @see ExampleContentAddressableStorageActor
pub struct ExampleContentAddressableStorage {
    content: Arc<RwLock<ExampleContentAddressableStorageContent>>,
}

impl ExampleContentAddressableStorage {
    pub fn new() -> Result<ExampleContentAddressableStorage, HolochainError> {
        Ok(ExampleContentAddressableStorage {
            content: Arc::new(RwLock::new(ExampleContentAddressableStorageContent::new())),
        })
    }
}

pub fn test_content_addressable_storage() -> ExampleContentAddressableStorage {
    ExampleContentAddressableStorage::new().expect("could not build example cas")
}

impl ContentAddressableStorage for ExampleContentAddressableStorage {
    fn add(&mut self, content: &AddressableContent) -> Result<(), HolochainError> {
        self.content
            .write()
            .unwrap()
            .unthreadable_add(&content.address(), &content.content())
    }

    fn contains(&self, address: &Address) -> Result<bool, HolochainError> {
        self.content.read().unwrap().unthreadable_contains(address)
    }

    fn fetch<AC: AddressableContent>(
        &self,
        address: &Address,
    ) -> Result<Option<AC>, HolochainError> {
        let content = self.content.read().unwrap().unthreadable_fetch(address)?;
        Ok(match content {
            Some(c) => Some(AC::from_content(&c)),
            None => None,
        })
    }
}

/// Not thread-safe CAS implementation with a HashMap
pub struct ExampleContentAddressableStorageContent {
    storage: HashMap<Address, Content>,
}

impl ExampleContentAddressableStorageContent {
    pub fn new() -> ExampleContentAddressableStorageContent {
        ExampleContentAddressableStorageContent {
            storage: HashMap::new(),
        }
    }

    fn unthreadable_add(
        &mut self,
        address: &Address,
        content: &Content,
    ) -> Result<(), HolochainError> {
        self.storage.insert(address.clone(), content.clone());
        Ok(())
    }

    fn unthreadable_contains(&self, address: &Address) -> Result<bool, HolochainError> {
        Ok(self.storage.contains_key(address))
    }

    fn unthreadable_fetch(&self, address: &Address) -> Result<Option<Content>, HolochainError> {
        Ok(self.storage.get(address).cloned())
    }
}

//A struct for our test suite that infers a type of ContentAddressableStorage
pub struct StorageTestSuite<T>
where
    T: ContentAddressableStorage,
{
    pub cas: T,
    /// it is important that every cloned copy of any CAS has a consistent view to data
    pub cas_clone: T,
}

impl<T> StorageTestSuite<T>
where
    T: ContentAddressableStorage + 'static,
{
    pub fn new(cas: T) -> StorageTestSuite<T> {
        StorageTestSuite {
            cas_clone: cas.clone(),
            cas: cas,
        }
    }

    //does round trip test that can infer two Addressable Content Types
    pub fn round_trip_test<Addressable, OtherAddressable>(
        mut self,
        content: Content,
        other_content: Content,
    ) where
        Addressable: AddressableContent + Clone + PartialEq + Debug,
        OtherAddressable: AddressableContent + Clone + PartialEq + Debug,
    {
        // based on associate type we call the right from_content function
        let addressable_content = Addressable::from_content(&content);
        let other_addressable_content = OtherAddressable::from_content(&other_content);

        // do things that would definitely break if cloning would show inconsistent data
        let both_cas = vec![self.cas.clone(), self.cas_clone.clone()];

        for cas in both_cas.iter() {
            assert_eq!(Ok(false), cas.contains(&addressable_content.address()));
            assert_eq!(
                Ok(None),
                cas.fetch::<Addressable>(&addressable_content.address())
            );
            assert_eq!(
                Ok(false),
                cas.contains(&other_addressable_content.address())
            );
            assert_eq!(
                Ok(None),
                cas.fetch::<OtherAddressable>(&other_addressable_content.address())
            );
        }

        // round trip some AddressableContent through the ContentAddressableStorage
        assert_eq!(Ok(()), self.cas.add(&content));

        for cas in both_cas.iter() {
            assert_eq!(Ok(true), cas.contains(&content.address()));
            assert_eq!(Ok(false), cas.contains(&other_content.address()));
            assert_eq!(Ok(Some(content.clone())), cas.fetch(&content.address()));
        }

        // multiple types of AddressableContent can sit in a single ContentAddressableStorage
        // the safety of this is only as good as the hashing algorithm(s) used
        assert_eq!(Ok(()), self.cas_clone.add(&other_content));

        for cas in both_cas.iter() {
            assert_eq!(Ok(true), cas.contains(&content.address()));
            assert_eq!(Ok(true), cas.contains(&other_content.address()));
            assert_eq!(Ok(Some(content.clone())), cas.fetch(&content.address()));
            assert_eq!(
                Ok(Some(other_content.clone())),
                cas.fetch(&other_content.address())
            );
        }

        // show consistent view on data across threads

        let entry = test_entry_unique();

        // initially should not find entry
        let thread_cas = self.cas.clone();
        let thread_entry = entry.clone();
        let (tx1, rx1) = channel();
        thread::spawn(move || {
            assert_eq!(
                None,
                thread_cas
                    .fetch::<Entry>(&thread_entry.address())
                    .expect("could not fetch from cas")
            );
            tx1.send(true).unwrap();
        });

        // should be able to add an entry found in the next channel
        let mut thread_cas = self.cas.clone();
        let thread_entry = entry.clone();
        let (tx2, rx2) = channel();
        thread::spawn(move || {
            rx1.recv().unwrap();
            thread_cas
                .add(&thread_entry)
                .expect("could not add entry to cas");
            tx2.send(true).expect("could not kick off next thread");
        });

        let thread_cas = self.cas.clone();
        let thread_entry = entry.clone();
        let handle = thread::spawn(move || {
            rx2.recv().unwrap();
            assert_eq!(
                Some(thread_entry.clone()),
                thread_cas
                    .fetch(&thread_entry.address())
                    .expect("could not fetch from cas")
            )
        });

        handle.join().unwrap();
    }
}

pub struct EavTestSuite;

impl EavTestSuite {
    pub fn test_round_trip(
        mut eav_storage: impl EntityAttributeValueStorage,
        entity_content: impl AddressableContent,
        attribute: String,
        value_content: impl AddressableContent,
    ) {
        let eav = EntityAttributeValue::new(
            &entity_content.address(),
            &"favourite-color".to_string(),
            &value_content.address(),
        );

        let two_stores = vec![eav_storage.clone(), eav_storage.clone()];

        for eav_storage in two_stores.iter() {
            assert_eq!(
                HashSet::new(),
                eav_storage
                    .fetch_eav(
                        Some(entity_content.address()),
                        Some(attribute.clone()),
                        Some(value_content.address())
                    )
                    .expect("could not fetch eav"),
            );
        }

        eav_storage.add_eav(&eav).expect("could not add eav");

        let mut expected = HashSet::new();
        expected.insert(eav.clone());

        for eav_storage in two_stores.iter() {
            // some examples of constraints that should all return the eav
            for (e, a, v) in vec![
                // constrain all
                (
                    Some(entity_content.address()),
                    Some(attribute.clone()),
                    Some(value_content.address()),
                ),
                // open entity
                (None, Some(attribute.clone()), Some(value_content.address())),
                // open attribute
                (
                    Some(entity_content.address()),
                    None,
                    Some(value_content.address()),
                ),
                // open value
                (
                    Some(entity_content.address()),
                    Some(attribute.clone()),
                    None,
                ),
                // open
                (None, None, None),
            ] {
                assert_eq!(
                    expected,
                    eav_storage.fetch_eav(e, a, v).expect("could not fetch eav"),
                );
            }
        }
    }
    pub fn test_one_to_many<A, S>(mut eav_storage: S)
    where
        A: AddressableContent + Clone,
        S: EntityAttributeValueStorage,
    {
        let one = A::from_content(&"foo".to_string());
        // it can reference itself, why not?
        let many_one = A::from_content(&"foo".to_string());
        let many_two = A::from_content(&"bar".to_string());
        let many_three = A::from_content(&"baz".to_string());
        let attribute = "one_to_many".to_string();

        let mut expected = HashSet::new();
        for many in vec![many_one.clone(), many_two.clone(), many_three.clone()] {
            let eav = EntityAttributeValue::new(&one.address(), &attribute, &many.address());
            eav_storage.add_eav(&eav).expect("could not add eav");
            expected.insert(eav);
        }

        // throw an extra thing referencing many to show fetch ignores it
        let two = A::from_content(&"foo".to_string());
        for many in vec![many_one.clone(), many_three.clone()] {
            eav_storage
                .add_eav(&EntityAttributeValue::new(
                    &two.address(),
                    &attribute,
                    &many.address(),
                ))
                .expect("could not add eav");
        }

        // show the many results for one
        assert_eq!(
            expected,
            eav_storage
                .fetch_eav(Some(one.address()), Some(attribute.clone()), None)
                .expect("could not fetch eav"),
        );

        // show one for the many results
        for many in vec![many_one.clone(), many_two.clone(), many_three.clone()] {
            let mut expected_one = HashSet::new();
            expected_one.insert(EntityAttributeValue::new(
                &one.address(),
                &attribute.clone(),
                &many.address(),
            ));
            assert_eq!(
                expected_one,
                eav_storage
                    .fetch_eav(None, Some(attribute.clone()), Some(many.address()))
                    .expect("could not fetch eav"),
            );
        }
    }

    pub fn test_many_to_one<A, S>(mut eav_storage: S)
    where
        A: AddressableContent + Clone,
        S: EntityAttributeValueStorage,
    {
        let one = A::from_content(&"foo".to_string());
        // it can reference itself, why not?
        let many_one = A::from_content(&"foo".to_string());
        let many_two = A::from_content(&"bar".to_string());
        let many_three = A::from_content(&"baz".to_string());
        let attribute = "many_to_one".to_string();

        let mut expected = HashSet::new();
        for many in vec![many_one.clone(), many_two.clone(), many_three.clone()] {
            let eav = EntityAttributeValue::new(&many.address(), &attribute, &one.address());
            eav_storage.add_eav(&eav).expect("could not add eav");
            expected.insert(eav);
        }

        // throw an extra thing referenced by many to show fetch ignores it
        let two = A::from_content(&"foo".to_string());
        for many in vec![many_one.clone(), many_three.clone()] {
            eav_storage
                .add_eav(&EntityAttributeValue::new(
                    &many.address(),
                    &attribute,
                    &two.address(),
                ))
                .expect("could not add eav");
        }

        // show the many referencing one
        assert_eq!(
            expected,
            eav_storage
                .fetch_eav(None, Some(attribute.clone()), Some(one.address()))
                .expect("could not fetch eav"),
        );

        // show one for the many results
        for many in vec![many_one.clone(), many_two.clone(), many_three.clone()] {
            let mut expected_one = HashSet::new();
            expected_one.insert(EntityAttributeValue::new(
                &many.address(),
                &attribute.clone(),
                &one.address(),
            ));
            assert_eq!(
                expected_one,
                eav_storage
                    .fetch_eav(Some(many.address()), Some(attribute.clone()), None)
                    .expect("could not fetch eav"),
            );
        }
    }
}

#[cfg(test)]
pub mod tests {
    use cas::{
        content::{ExampleAddressableContent, OtherExampleAddressableContent},
        storage::{test_content_addressable_storage, StorageTestSuite},
    };

    /// show that content of different types can round trip through the same storage
    #[test]
    fn example_content_round_trip_test() {
        let test_suite = StorageTestSuite::new(test_content_addressable_storage());
        test_suite.round_trip_test::<ExampleAddressableContent, OtherExampleAddressableContent>(
            String::from("foo"),
            String::from("bar"),
        );
    }
}
