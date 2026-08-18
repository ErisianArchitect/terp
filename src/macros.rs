
#[macro_export]
macro_rules! bitflags {
    (@reserved_err: $name:ident) => {
        compile_error!(concat!(
            "`",
            stringify!($name),
            "` is a reserved name.",
        ));
    };
    (@check_name: NONE) => { $crate::bitflags!{@reserved_err: NONE} };
    (@check_name: ALL) => { $crate::bitflags!{@reserved_err: ALL} };
    (@check_name: ANY) => { $crate::bitflags!{@reserved_err: ANY} };
    (@check_name: EQ) => { $crate::bitflags!{@reserved_err: EQ} };
    (@check_name: NE) => { $crate::bitflags!{@reserved_err: NE} };
    (@check_name: $allowed:tt) => {};
    (
        $(
            #[$attr:meta]
        )*
        $struct_vis:vis struct $type_name:ident ($mask_type:ty) {
            $(
                $flag_vis:vis $flag_name:ident = $mask_value:expr
            ),+
            $(,)?
        }
    ) => {
        paste::paste!{
            $(
                $crate::bitflags!{@check_name: [< $flag_name:snake:upper >]}
            )*
        }
        $(#[$attr])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        $struct_vis struct $type_name ($mask_type);
        paste::paste!{
            impl $type_name {
                #[must_use]
                #[inline(always)]
                pub const fn new() -> Self {
                    Self::NONE
                }

                #[must_use]
                #[inline(always)]
                pub const fn is_empty(self) -> bool {
                    self.0 == 0
                }

                #[must_use]
                #[inline(always)]
                pub const fn eq(self, other: Self) -> bool {
                    self.0 == other.0
                }

                #[must_use]
                #[inline(always)]
                pub const fn ne(self, other: Self) -> bool {
                    self.0 != other.0
                }

                #[must_use]
                #[inline(always)]
                pub const fn or(self, other: Self) -> Self {
                    Self(self.0 | other.0)
                }

                #[inline(always)]
                pub const fn or_eq(&mut self, other: Self) -> &mut Self {
                    self.0 |= other.0;
                    self
                }

                #[must_use]
                #[inline(always)]
                pub const fn not(self) -> Self {
                    Self(!self.0)
                }

                #[must_use]
                #[inline(always)]
                pub const fn and(self, other: Self) -> Self {
                    Self(self.0 & other.0)
                }

                #[inline(always)]
                pub const fn and_eq(&mut self, other: Self) -> &mut Self {
                    self.0 &= other.0;
                    self
                }

                #[must_use]
                #[inline(always)]
                pub const fn xor(self, other: Self) -> Self {
                    Self(self.0 ^ other.0)
                }

                #[inline(always)]
                pub const fn xor_eq(&mut self, other: Self) -> &mut Self {
                    self.0 ^= other.0;
                    self
                }

                pub const fn union(flags: &[Self]) -> Self {
                    let Some(end) = flags.len().checked_sub(1) else {
                        return Self(0);
                    };
                    let mut i = 0;
                    let mut builder = Self(0);
                    loop {
                        builder.add(flags[i]);
                        if i == end {
                            break;
                        }
                        i += 1;
                    }
                    builder
                }

                #[inline(always)]
                pub const fn add(&mut self, other: Self) -> &mut Self {
                    self.or_eq(other)
                }

                #[inline(always)]
                pub const fn remove(&mut self, other: Self) -> &mut Self {
                    self.and_eq(other.not())
                }

                pub const fn add_all(&mut self, flags: &[Self]) -> &mut Self {
                    let Some(end) = flags.len().checked_sub(1) else {
                        return self;
                    };
                    let mut i = 0;
                    loop {
                        self.add(flags[i]);
                        if i >= end {
                            break;
                        }
                        i += 1;
                    }
                    self
                }

                pub const fn remove_all(&mut self, flags: &[Self]) -> &mut Self {
                    let Some(end) = flags.len().checked_sub(1) else {
                        return self;
                    };
                    let mut i = 0;
                    loop {
                        self.remove(flags[i]);
                        if i >= end {
                            break;
                        }
                        i += 1;
                    }
                    self
                }

                #[must_use]
                #[inline(always)]
                pub const fn with(self, other: Self) -> Self {
                    self.or(other)
                }

                #[must_use]
                #[inline(always)]
                pub const fn without(self, other: Self) -> Self {
                    let mut me = self;
                    me.remove(other);
                    me
                }

                #[must_use]
                #[inline(always)]
                pub const fn with_all(self, flags: &[Self]) -> Self {
                    let mut me = self;
                    me.add_all(flags);
                    me
                }

                #[must_use]
                #[inline(always)]
                pub const fn without_all(self, flags: &[Self]) -> Self {
                    let mut me = self;
                    me.remove_all(flags);
                    me
                }

                #[inline(always)]
                pub const fn has_any(self, flags: Self) -> bool {
                    self.0 & flags.0 != 0
                }

                #[inline(always)]
                pub const fn has_all(self, flags: Self) -> bool {
                    self.0 & flags.0 == flags.0
                }

                #[inline(always)]
                pub const fn has_none(self, flags: Self) -> bool {
                    self.0 & flags.0 == 0
                }

                pub const NONE: Self = Self(0);
                pub const ALL: Self = Self(0 $( | ($mask_value) )*);
                paste::paste!{
                    $(
                        $flag_vis const [< $flag_name:snake:upper >]: Self = Self($mask_value);

                        #[must_use]
                        #[inline(always)]
                        pub const fn [< $flag_name:snake:lower >](self) -> bool {
                            self.has_all(Self::[< $flag_name:snake:upper >])
                        }

                        #[inline(always)]
                        pub const fn [< and_ $flag_name:snake:lower >](self) -> Self {
                            self.and(Self::[< $flag_name:snake:upper >])
                        }

                        #[inline(always)]
                        pub const fn [< or_ $flag_name:snake:lower >](self) -> Self {
                            self.or(Self::[< $flag_name:snake:upper >])
                        }

                        #[inline(always)]
                        pub const fn [< xor_ $flag_name:snake:lower >](self) -> Self {
                            self.xor(Self::[< $flag_name:snake:upper >])
                        }

                        #[inline(always)]
                        pub const fn [< add_ $flag_name:snake:lower >](&mut self) -> &mut Self {
                            self.add(Self::[< $flag_name:snake:upper >])
                        }

                        #[inline(always)]
                        pub const fn [< remove_ $flag_name:snake:lower >](&mut self) -> &mut Self {
                            self.remove(Self::[< $flag_name:snake:upper >])
                        }

                        #[must_use]
                        #[inline(always)]
                        pub const fn [< with_ $flag_name:snake:lower >](self) -> Self {
                            self.with(Self::[< $flag_name:snake:upper >])
                        }

                        #[must_use]
                        #[inline(always)]
                        pub const fn [< without_ $flag_name:snake:lower >](self) -> Self {
                            self.without(Self::[< $flag_name:snake:upper >])
                        }
                    )*
                }
            }
        }
    }
}
