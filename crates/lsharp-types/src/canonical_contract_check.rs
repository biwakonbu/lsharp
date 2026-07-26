//! canonical metadata contract の検査。

mod non_vacuity;
#[cfg(test)]
mod tests;
mod types;

pub(crate) use non_vacuity::{
    check_assertion_non_vacuity, check_case_non_vacuity, check_property_non_vacuity,
};
pub(crate) use types::{check_assertion_types, check_case_types, check_property_types};
