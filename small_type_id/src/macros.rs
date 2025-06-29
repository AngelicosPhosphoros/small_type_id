#[doc(hidden)]
#[macro_export]
macro_rules! private_macro_implement_type_and_register {
    ($tname:ident, $name_literal:literal, $seed:literal) => {
        const _: () = {
            $crate::private::implement_type_id!($tname, $name_literal, $seed);
            $crate::private::register_type_id!($tname, $name_literal);
        };
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_macro_implement_type_id {
    ($tname:ident, $name_literal:literal, $seed:literal) => {
        unsafe impl $crate::HasTypeId for $tname {
            const TYPE_ID: $crate::TypeId = {
                const INPUT_LEN: usize = $crate::private::compute_input_len(
                    ::core::concat!(::core::module_path!(), "::", $name_literal),
                    ::core::option_env!("CARGO_PKG_VERSION"),
                );
                $crate::private::compute_id::<INPUT_LEN>(
                    ::core::concat!(::core::module_path!(), "::", $name_literal),
                    ::core::option_env!("CARGO_PKG_VERSION"),
                    $seed,
                )
            };
        }
    };
}

#[doc(hidden)]
#[macro_export]
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
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

#[doc(hidden)]
#[macro_export]
#[cfg(any(target_os = "windows", target_os = "linux"))]
macro_rules! private_macro_register_type_id {
    ($tname:ident, $name_literal:literal) => {
        #[unsafe(link_section=$crate::private::link_section_name!())]
        #[used]
        static ENTRY: $crate::private::TypeEntry = $crate::private::TypeEntry::new(
            ::core::concat!(::core::module_path!(), "::", $name_literal),
            <$tname as ::small_type_id::HasTypeId>::TYPE_ID,
        );
    };
}

// This macro is needed to make every link_section attribute distinct
// in case of multiple crate versions being linked.
// We cannot just use `env!("CARGO_PKG_VERSION")` because it works on caller site.
#[doc(hidden)]
#[macro_export]
macro_rules! private_macro_small_type_id_version {
    () => {
        // Use underscores instead of dots because otherwise
        // linker doesn't define `__start_<sectionname>` and `__stop_<sectionname>`
        // variables.
        "0_0_1_alpha"
    };
}

#[doc(hidden)]
#[macro_export]
#[cfg(target_os = "windows")]
macro_rules! private_macro_link_section_name {
    () => {
        ::core::concat!(
            "smltidrs_small_type_id_rs$",
            $crate::private::small_type_id_version!(),
            "_b"
        )
    };
}

#[doc(hidden)]
#[macro_export]
#[cfg(target_os = "linux")]
macro_rules! private_macro_link_section_name {
    () => {
        ::core::concat!(
            "smltidrs_small_type_id_rs",
            $crate::private::small_type_id_version!(),
        )
    };
}

#[doc(hidden)]
#[macro_export]
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
macro_rules! private_macro_link_section_name {
    () => {
        ::core::compile_error!("Usage of link section is not supported on current platform (yet).")
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn verify_version_macro() {
        let made_linkable: String = env!("CARGO_PKG_VERSION")
            .chars()
            .map(|x| if x.is_alphanumeric() { x } else { '_' })
            .collect();
        assert_eq!(made_linkable, crate::private::small_type_id_version!());
    }
}
