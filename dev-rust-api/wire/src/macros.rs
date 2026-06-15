//! C-ABI declaration macros.

/// Declares a C-ABI compatible struct and acts as a marker for the Python SDK code generator.
/// 
/// Enforces `#[repr(C)]` and derives `bytemuck::Pod` and `bytemuck::Zeroable`.
#[macro_export]
macro_rules! decl_wire {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field_name:ident: $field_type:ty
            ),* $(,)?
        }
    ) => {
        #[repr(C)]
        #[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        $(#[$meta])*
        $vis struct $name {
            $(
                $(#[$field_meta])*
                $field_vis $field_name: $field_type,
            )*
        }
    };
}
