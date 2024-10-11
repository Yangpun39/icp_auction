use candid::{CandidType, Decode, Deserialize, Encode};
use ic_cdk_macros::export_candid;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};


#[derive(CandidType, Deserialize)]
struct Item {
    description: String,
    is_active: bool,
    bid_count:u32,
    highest_bid:u64,
    highest_bidder:candid::Principal,
    voted: Vec<candid::Principal>,
    owner: candid::Principal,
}

#[derive(CandidType, Deserialize)]
struct Createitem {
    description: String,
    is_active: bool,
}

#[derive(CandidType, Deserialize)]
enum BidError {
    AlreadyBid,
    ItemNotActive,
    Unauthorized,
    NoItem,
    UpdateError,
    BidFailed,
}

impl Storable for Item{
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

type Memory = VirtualMemory<DefaultMemoryImpl>;
const MAX_VALUE_SIZE: u32 = 100;

// Implement BoundedStorable for Item
impl BoundedStorable for Item {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE; // Adjust the size as needed
    const IS_FIXED_SIZE: bool = false;
}

// Initialize the Items map with a new MemoryId
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
    RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static ITEM_MAP: RefCell<StableBTreeMap<u64, Item, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))), // Use a different MemoryId if needed
        )
    );
}


#[ic_cdk_macros::query]
fn get_item(key: u64) -> Option<Item> {
    ITEM_MAP.with(|p| p.borrow().get(&key))
}

#[ic_cdk_macros::query]
fn get_item_count() -> u64 {
    ITEM_MAP.with(|p| p.borrow().len())
}

#[ic_cdk_macros::update]
fn create_item(key: u64, item: Createitem) -> Option<Item> { 
    let value = Item {
        description: item.description,
        bid_count: 0u32,
        highest_bid:0u64,
        highest_bidder:ic_cdk::caller(),
        is_active: item.is_active,
        voted: vec![],
        owner: ic_cdk::caller(),
    };
    ITEM_MAP.with(|p| p.borrow_mut().insert(key, value))
}
#[ic_cdk_macros::update]
fn bid(key: u64, amount: u64) -> Result<String, BidError> {
    ITEM_MAP.with(|p| {
        let mut item = match p.borrow().get(&key) {
            Some(value) => value,
            None => return Err(BidError::NoItem),
        };
        if !item.is_active {
            return Err(BidError::ItemNotActive);
        }
            if !item.voted.contains(&ic_cdk::caller()) {
                if amount > item.highest_bid {
                    item.highest_bid = amount;
                    item.highest_bidder = ic_cdk::caller();
                    item.bid_count += 1;
                    item.voted.push(ic_cdk::caller()); 
                    p.borrow_mut().insert(key, item);
        
                    Ok(format!("Bid placed successfully. New highest bid: {}", amount))
                }
                else {
                    item.bid_count += 1;
                    item.voted.push(ic_cdk::caller()); 
                    p.borrow_mut().insert(key, item);
                    Ok(format!("Bid amount too low, but you have been added to the voted list."))
                }
            } else {
                return Err(BidError::AlreadyBid);
            }

           
    })
}
#[ic_cdk_macros::update]
fn remove(key: u64) -> Result<String, BidError> {
    ITEM_MAP.with(|p|{
        let mut item = match p.borrow().get(&key) {
            Some(value) => value,
            None => return Err(BidError::NoItem),
        };
        if !item.is_active {
            return Err(BidError::ItemNotActive);
        }
        if ic_cdk::caller() == item.owner{
            item.is_active=false;
            item.owner=item.highest_bidder;
            p.borrow_mut().insert(key, item);

            Ok(format!("Bid placed successfully"))
        }else {
            return Err(BidError::Unauthorized)
        }
    })
}

#[ic_cdk_macros::query]
fn get_all_items() -> Vec<Item> {
    ITEM_MAP.with(|p| {
        p.borrow()
            .iter()
            .map(|(_key, item)| item) 
            .collect()
    })
}
#[ic_cdk_macros::query]
fn get_item_sold_for_most() -> Option<Item> {
    ITEM_MAP.with(|p| {
        let mut highest_bid_item: Option<Item> = None;
        let mut max_bid = 0u64;

        for (_key, item) in p.borrow().iter() {
            if item.highest_bid > max_bid {
                max_bid = item.highest_bid;
                highest_bid_item = Some(item);
            }
        }

        highest_bid_item
    })
}
#[ic_cdk_macros::query]
fn get_item_most_bids() -> Option<Item> {
    ITEM_MAP.with(|p| {
        let mut most_bids_item: Option<Item> = None;
        let mut max_bids = 0u32;

        for (_key, item) in p.borrow().iter() {
            if item.bid_count > max_bids {
                max_bids = item.bid_count;
                most_bids_item = Some(item);
            }
        }

        most_bids_item
    })
}

// #[ic_cdk_macros::update]
// fn edit_Item(key: u64, Item: CreateItem) -> Result<(), VoteError> {
//     Item_MAP.with(|p| {
//         let old_Item = match p.borrow().get(&key) {
//             Some(value) => value,
//             None => return Err(VoteError::NoItem),
//         };
//         if ic_cdk::caller() != old_Item.owner {            
//             return Err(VoteError::Unauthorized);
//         }
//         let value = Item {
//             description: Item.description,
//             approve: old_Item.approve,
//             reject: old_Item.reject,
//             pass: old_Item.pass,
//             is_active: Item.is_active,
//             voted: old_Item.voted,
//             owner: ic_cdk::caller(),
//         };
//         let res = p.borrow_mut().insert(key, value);
//         match res {
//             Some(_) => Ok(()),
//             None => Err(VoteError::UpdateError),
//         }
//     })
// }

// #[ic_cdk_macros::update]
// fn end_Item(key: u64) -> Result<(), VoteError> {
//     Item_MAP.with(|p| {
//         let mut Item = p.borrow_mut().get(&key).unwrap();
//         if ic_cdk::caller() != Item.owner {
//             return Err(VoteError::Unauthorized);
//         }
//         Item.is_active = false;
//         let res = p.borrow_mut().insert(key, Item);
//         match res {
//             Some(_) => Ok(()),
//             None => Err(VoteError::UpdateError),
//         }
//     })
// }

// #[ic_cdk_macros::update]
// fn vote_(key: u64, choice: VoteTypes) -> Result<(), VoteError> {
//     Item_MAP.with(|p| {
//         let mut Item = p.borrow_mut().get(&key).unwrap();
//         let caller = ic_cdk::caller();
//         if Item.voted.contains(&caller) {
//             return Err(VoteError::AlreadyVoted);
//         } else if !Item.is_active {
//             return Err(VoteError::ItemNotActive);
//         }
//         match choice {
//             VoteTypes::Approve => Item.approve += 1,
//             VoteTypes::Reject => Item.reject += 1,
//             VoteTypes::Pass => Item.pass += 1,
//         }
//         Item.voted.push(caller);
//         let res = p.borrow_mut().insert(key, Item);
//         match res {
//             Some(_) => Ok(()),
//             None => Err(VoteError::VoteFailed),
//         }
//     })
// }

// #[ic_cdk::query]
// fn get_Item_status(key: u64) -> String {
//     let Item = Item_MAP.with(|p| p.borrow().get(&key));

//     match Item {
//         Some(Item) => {
//             // Check if the Item has at least 5 votes
//             if Item.approve + Item.reject + Item.pass < 5 {
//                 return String::from("UNDECIDED");
//             }

//             let total_votes = Item.approve + Item.reject + Item.pass;

//             // Calculate the majority condition (at least 50% of votes)
//             let majority_condition = total_votes / 2;

//             // Determine the status based on votes
//             if Item.approve >= majority_condition {
//                 return String::from("APPROVED");
//             } else if Item.reject >= majority_condition {
//                 return String::from("REJECTED");
//             } else if Item.pass >= majority_condition {
//                 return String::from("PASSED");
//             } else {
//                 return String::from("UNDECIDED");
//             }
//         }
//         None => String::from("NO_Item"),
//     }
// }



export_candid!();