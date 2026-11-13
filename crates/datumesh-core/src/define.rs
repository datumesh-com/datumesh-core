//! The define-time check of a function descriptor under a name (2.5, section 7,
//! `spec-0.5.0`).

use crate::descriptor::FunctionDescriptor;
use crate::error::{err, index, ErrorCode, Result};
use crate::lexical;

/// The checks define makes before it stores a function, in this order, reporting the first
/// failure: `name` is a full function name (2.3, `E_CANON_NAME`); `descriptor_json` is a
/// function descriptor (section 7, with the codes of [`FunctionDescriptor::from_json`]); and
/// a function under a rooted namespace names no function under an unrooted one in its
/// `inputs`, at the first such input in `inputs` order (`E_UNROOTED_INPUT`, 2.5). An unrooted
/// function may read both. Formula descriptors' own define-time rules (15.6) are not checked
/// here. Returns the descriptor, whose canonical bytes and hash are the vector's outputs.
pub fn check_define(name: &str, descriptor_json: &[u8]) -> Result<FunctionDescriptor> {
    lexical::check_function_name(name, "$.name")?;
    let descriptor = FunctionDescriptor::from_json(descriptor_json)?;
    if !lexical::is_unrooted(name) {
        if let Some(i) = descriptor.inputs.iter().position(|input| lexical::is_unrooted(input)) {
            return err(
                ErrorCode::UnrootedInput,
                &index("$.inputs", i),
                format!("the rooted function {name:?} reads the unrooted function {:?}", descriptor.inputs[i]),
            );
        }
    }
    Ok(descriptor)
}
