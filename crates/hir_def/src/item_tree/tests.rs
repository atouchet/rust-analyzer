use base_db::fixture::WithFixture;
use expect_test::{expect, Expect};

use crate::{db::DefDatabase, test_db::TestDB};

fn check(ra_fixture: &str, expect: Expect) {
    let (db, file_id) = TestDB::with_single_file(ra_fixture);
    let item_tree = db.file_item_tree(file_id.into());
    let pretty = item_tree.pretty_print();
    expect.assert_eq(&pretty);
}

#[test]
fn imports() {
    check(
        r#"
//! file comment
#![no_std]
//! another file comment

extern crate self as renamed;
pub(super) extern crate bli;

pub use crate::path::{nested, items as renamed, Trait as _};
use globs::*;

/// docs on import
use crate::{A, B};

use a::{c, d::{e}};
        "#,
        expect![[r##"
            #![doc = " file comment"]  // AttrId { ast_index: 0 }
            #![no_std]  // AttrId { ast_index: 1 }
            #![doc = " another file comment"]  // AttrId { ast_index: 2 }

            pub(self) extern crate self as renamed;

            pub(super) extern crate bli;

            pub use crate::path::{nested, items as renamed, Trait as _};

            pub(self) use globs::*;

            #[doc = " docs on import"]  // AttrId { ast_index: 0 }
            pub(self) use crate::{A, B};

            pub(self) use a::{c, d::{e}};
        "##]],
    );
}

#[test]
fn extern_blocks() {
    check(
        r#"
#[on_extern_block]
extern "C" {
    #[on_extern_type]
    type ExType;

    #[on_extern_static]
    static EX_STATIC: u8;

    #[on_extern_fn]
    fn ex_fn();
}
        "#,
        expect![[r##"
            #[on_extern_block]  // AttrId { ast_index: 0 }
            extern "C" {
                #[on_extern_type]  // AttrId { ast_index: 0 }
                pub(self) type ExType;

                #[on_extern_static]  // AttrId { ast_index: 0 }
                pub(self) static EX_STATIC: u8 = _;

                #[on_extern_fn]  // AttrId { ast_index: 0 }
                pub(self) fn ex_fn() -> ();
            }
        "##]],
    );
}

#[test]
fn adts() {
    check(
        r#"
struct Unit;

#[derive(Debug)]
struct Struct {
    /// fld docs
    fld: (),
}

struct Tuple(#[attr] u8);

union Ize {
    a: (),
    b: (),
}

enum E {
    /// comment on Unit
    Unit,
    /// comment on Tuple
    Tuple(u8),
    Struct {
        /// comment on a: u8
        a: u8,
    }
}
        "#,
        expect![[r##"
            pub(self) struct Unit;

            #[derive(Debug)]  // AttrId { ast_index: 0 }
            pub(self) struct Struct {
                #[doc = " fld docs"]  // AttrId { ast_index: 0 }
                pub(self) fld: (),
            }

            pub(self) struct Tuple(
                #[attr]  // AttrId { ast_index: 0 }
                pub(self) 0: u8,
            );

            pub(self) union Ize {
                pub(self) a: (),
                pub(self) b: (),
            }

            pub(self) enum E {
                #[doc = " comment on Unit"]  // AttrId { ast_index: 0 }
                Unit,
                #[doc = " comment on Tuple"]  // AttrId { ast_index: 0 }
                Tuple(
                    pub(self) 0: u8,
                ),
                Struct {
                    #[doc = " comment on a: u8"]  // AttrId { ast_index: 0 }
                    pub(self) a: u8,
                },
            }
        "##]],
    );
}

#[test]
fn misc() {
    check(
        r#"
pub static mut ST: () = ();

const _: Anon = ();

#[attr]
fn f(#[attr] arg: u8, _: ()) {
    #![inner_attr_in_fn]
}

trait Tr: SuperTrait + 'lifetime {
    type Assoc: AssocBound = Default;
    fn method(&self);
}
        "#,
        expect![[r##"
            pub static mut ST: () = _;

            pub(self) const _: Anon = _;

            #[attr]  // AttrId { ast_index: 0 }
            #[inner_attr_in_fn]  // AttrId { ast_index: 1 }
            // flags = 0x2
            pub(self) fn f(
                #[attr]  // AttrId { ast_index: 0 }
                arg: u8,
                _: (),
            ) -> ();

            pub(self) trait Tr<Self>
            where
                Self: SuperTrait,
                Self: 'lifetime
            {
                pub(self) type Assoc: AssocBound = Default;

                // flags = 0x1
                pub(self) fn method(
                    _: &Self,
                ) -> ();
            }
        "##]],
    );
}

#[test]
fn modules() {
    check(
        r#"
/// outer
mod inline {
    //! inner

    use super::*;

    fn fn_in_module() {}
}

mod outline;
        "#,
        expect![[r##"
            #[doc = " outer"]  // AttrId { ast_index: 0 }
            #[doc = " inner"]  // AttrId { ast_index: 1 }
            pub(self) mod inline {
                pub(self) use super::*;

                // flags = 0x2
                pub(self) fn fn_in_module() -> ();
            }

            pub(self) mod outline;
        "##]],
    );
}

#[test]
fn macros() {
    check(
        r#"
macro_rules! m {
    () => {};
}

pub macro m2() {}

m!();
        "#,
        expect![[r#"
            macro_rules! m { ... }

            pub macro m2 { ... }

            m!(...);
        "#]],
    );
}

#[test]
fn mod_paths() {
    check(
        r#"
struct S {
    a: self::Ty,
    b: super::SuperTy,
    c: super::super::SuperSuperTy,
    d: ::abs::Path,
    e: crate::Crate,
    f: plain::path::Ty,
}
        "#,
        expect![[r#"
            pub(self) struct S {
                pub(self) a: self::Ty,
                pub(self) b: super::SuperTy,
                pub(self) c: super::super::SuperSuperTy,
                pub(self) d: ::abs::Path,
                pub(self) e: crate::Crate,
                pub(self) f: plain::path::Ty,
            }
        "#]],
    )
}

#[test]
fn types() {
    check(
        r#"
struct S {
    a: Mixed<'a, T, Item=(), OtherItem=u8>,
    b: <Fully as Qualified>::Syntax,
    c: <TypeAnchored>::Path::<'a>,
    d: dyn for<'a> Trait<'a>,
}
        "#,
        expect![[r#"
            pub(self) struct S {
                pub(self) a: Mixed<'a, T, Item = (), OtherItem = u8>,
                pub(self) b: Qualified<Self=Fully>::Syntax,
                pub(self) c: <TypeAnchored>::Path<'a>,
                pub(self) d: dyn for<'a> Trait<'a>,
            }
        "#]],
    )
}

#[test]
fn generics() {
    check(
        r#"
struct S<'a, 'b: 'a, T: Copy + 'a + 'b, const K: u8 = 0> {
    field: &'a &'b T,
}

struct Tuple<T: Copy, U: ?Sized>(T, U);

impl<'a, 'b: 'a, T: Copy + 'a + 'b, const K: u8 = 0> S<'a, 'b, T, K> {
    fn f<G: 'a>(arg: impl Copy) -> impl Copy {}
}

enum Enum<'a, T, const U: u8> {}
union Union<'a, T, const U: u8> {}

trait Tr<'a, T: 'a>: Super where Self: for<'a> Tr<'a, T> {}
        "#,
        expect![[r#"
            pub(self) struct S<'a, 'b, T, const K: u8>
            where
                T: Copy,
                T: 'a,
                T: 'b
            {
                pub(self) field: &'a &'b T,
            }

            pub(self) struct Tuple<T, U>(
                pub(self) 0: T,
                pub(self) 1: U,
            )
            where
                T: Copy,
                U: ?Sized;

            impl<'a, 'b, T, const K: u8> S<'a, 'b, T, K>
            where
                T: Copy,
                T: 'a,
                T: 'b
            {
                // flags = 0x2
                pub(self) fn f<G>(
                    arg: impl Copy,
                ) -> impl Copy
                where
                    G: 'a;
            }

            pub(self) enum Enum<'a, T, const U: u8> {
            }

            pub(self) union Union<'a, T, const U: u8> {
            }

            pub(self) trait Tr<'a, Self, T>
            where
                Self: Super,
                T: 'a,
                Self: for<'a> Tr<'a, T>
            {
            }
        "#]],
    )
}

#[test]
fn inherit_visibility() {
    check(
        r#"
pub(crate) enum En {
    Var1(u8),
    Var2 {
        fld: u8,
    },
}

pub(crate) trait Tr {
    fn f();
    fn method(&self) {}
}
        "#,
        expect![[r#"
            pub(crate) enum En {
                Var1(
                    pub(crate) 0: u8,
                ),
                Var2 {
                    pub(crate) fld: u8,
                },
            }

            pub(crate) trait Tr<Self> {
                pub(crate) fn f() -> ();

                // flags = 0x3
                pub(crate) fn method(
                    _: &Self,
                ) -> ();
            }
        "#]],
    )
}
