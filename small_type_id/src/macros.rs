#[doc(hidden)]
#[macro_export]
macro_rules! private_macro_implement_type_and_register {
    ($tname:ident, $name_literal:literal) => {
        const _: () = {
            $crate::private::implement_type_id!($tname, $name_literal);
            $crate::private::register_type_id!($tname, $name_literal);
            ()
        };
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_macro_implement_type_id {
    ($tname:ident, $name_literal:literal) => {
        unsafe impl $crate::HasTypeId for $tname {
            const TYPE_ID: $crate::TypeId = {
                const CRATE_VERSION: &::core::primitive::str =
                    match ::core::option_env!("CARGO_PKG_VERSION") {
                        ::core::option::Option::Some(x) => x,
                        ::core::option::Option::None => "",
                    };
                const MODULE_AND_NAME: &str =
                    ::core::concat!(::core::module_path!(), "::", $name_literal);
                if CRATE_VERSION.is_empty() {
                    ::small_type_id::private::compute_id(MODULE_AND_NAME.as_bytes())
                } else {
                    const SUM_LEN: usize = MODULE_AND_NAME.len() + 2 + CRATE_VERSION.len();
                    let concatenated: [u8; SUM_LEN] = ::small_type_id::private::concat_bytes(&[
                        MODULE_AND_NAME.as_bytes(),
                        b"::",
                        CRATE_VERSION.as_bytes(),
                    ]);
                    ::small_type_id::private::compute_id(&concatenated)
                }
            };
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_macro_register_type_id {
    ($tname:ident, $name_literal:literal) => {
        static ENTRY_0KKVMQVJV2BRIOQ8EILZ7: $crate::private::TypeEntry =
            $crate::private::TypeEntry::new(
                ::core::concat!(::core::module_path!(), "::", $name_literal),
                <$tname as ::small_type_id::HasTypeId>::TYPE_ID,
            );

        $crate::private::ctor! {
            #[ctor]
            #[inline]
            unsafe fn register_0kkvmqvjv2brioq8eilz7() {
                unsafe {
                    $crate::private::register_type(&ENTRY_0KKVMQVJV2BRIOQ8EILZ7);
                }
            }
        }
    };
}
