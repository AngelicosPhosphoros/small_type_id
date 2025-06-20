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
                const INPUT_LEN: usize = $crate::private::compute_input_len(
                    ::core::concat!(::core::module_path!(), "::", $name_literal),
                    ::core::option_env!("CARGO_PKG_VERSION"),
                );
                $crate::private::compute_id::<INPUT_LEN>(
                    ::core::concat!(::core::module_path!(), "::", $name_literal),
                    ::core::option_env!("CARGO_PKG_VERSION"),
                )
            };
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_macro_register_type_id {
    ($tname:ident, $name_literal:literal) => {
        static ENTRY: $crate::private::TypeEntry = $crate::private::TypeEntry::new(
            ::core::concat!(::core::module_path!(), "::", $name_literal),
            <$tname as ::small_type_id::HasTypeId>::TYPE_ID,
        );

        $crate::private::ctor! {
            #[ctor]
            #[inline]
            unsafe fn register_0kkvmqvjv2brioq8eilz7() {
                unsafe {
                    $crate::private::register_type(&ENTRY);
                }
            }
        }
    };
}
