extern crate holochain_core_types;
extern crate holochain_wasm_utils;

use holochain_core_types::hash::HashString;
use holochain_wasm_utils::{
  api_serialization::commit::{CommitEntryArgs, CommitEntryResult},
  memory_allocation::*, memory_serialization::*
};

extern {
  fn hc_commit_entry(encoded_allocation_of_input: i32) -> i32;
}

//-------------------------------------------------------------------------------------------------
// HC Commit Function Call - Succesful
//-------------------------------------------------------------------------------------------------

/// Call HC API COMMIT function with proper input struct
/// return hash of entry added source chain
fn hdk_commit(mem_stack: &mut SinglePageStack, entry_type_name: &str, entry_value: &str)
  -> Result<String, String>
{
  // Put args in struct and serialize into memory
  let input = CommitEntryArgs {
    entry_type_name: entry_type_name.to_owned(),
    entry_value: entry_value.to_owned(),
  };
  let maybe_allocation =  serialize(mem_stack, input);
  if let Err(return_code) = maybe_allocation {
    return Err(return_code.to_string());
  }
  let allocation_of_input = maybe_allocation.unwrap();

  // Call WASMI-able commit
  let encoded_allocation_of_result: i32;
  unsafe {
    encoded_allocation_of_result = hc_commit_entry(allocation_of_input.encode() as i32);
  }
  // Deserialize complex result stored in memory
  let output: CommitEntryResult = try_deserialize_allocation(encoded_allocation_of_result as u32)?;

  // Free result & input allocations and all allocations made inside commit()
  mem_stack.deallocate(allocation_of_input).expect("deallocate failed");

  // Return hash
  Ok(output.address.to_string())
}


//-------------------------------------------------------------------------------------------------
// HC COMMIT Function Call - Fail
//-------------------------------------------------------------------------------------------------

// Simulate error in commit function by inputing output struct as input
fn hdk_commit_fail(mem_stack: &mut SinglePageStack)
  -> Result<String, String>
{
  // Put args in struct and serialize into memory
  let input = CommitEntryResult {
    address: HashString::from("whatever"),
    validation_failure: String::from("")
  };
  let maybe_allocation =  serialize(mem_stack, input);
  if let Err(return_code) = maybe_allocation {
    return Err(return_code.to_string());
  }
  let allocation_of_input = maybe_allocation.unwrap();

  // Call WASMI-able commit
  let encoded_allocation_of_result: i32;
  unsafe {
    encoded_allocation_of_result = hc_commit_entry(allocation_of_input.encode() as i32);
  }
  // Deserialize complex result stored in memory
  let output: CommitEntryResult = try_deserialize_allocation(encoded_allocation_of_result as u32)?;

  // Free result & input allocations and all allocations made inside commit()
  mem_stack.deallocate(allocation_of_input).expect("deallocate failed");

  // Return hash
  Ok(output.address.to_string())
}


//-------------------------------------------------------------------------------------------------
//  Exported functions with required signature (=pointer to serialized complex parameter)
//-------------------------------------------------------------------------------------------------

/// Function called by Holochain Instance
/// encoded_allocation_of_input : encoded memory offset and length of the memory allocation
/// holding input arguments
/// returns encoded allocation used to store output
#[no_mangle]
pub extern "C" fn test(encoded_allocation_of_input: usize) -> i32 {
  let mut mem_stack = SinglePageStack::from_encoded(encoded_allocation_of_input as u32);
  let output = hdk_commit(&mut mem_stack, "testEntryType", "hello");
  return serialize_into_encoded_allocation(&mut mem_stack, output);
}

/// Function called by Holochain Instance
/// encoded_allocation_of_input : encoded memory offset and length of the memory allocation
/// holding input arguments
/// returns encoded allocation used to store output
#[no_mangle]
pub extern "C" fn test_fail(encoded_allocation_of_input: usize) -> i32 {
  let mut mem_stack = SinglePageStack::from_encoded(encoded_allocation_of_input as u32);
  let output = hdk_commit_fail(&mut mem_stack);
  return serialize_into_encoded_allocation(&mut mem_stack, output);
}
