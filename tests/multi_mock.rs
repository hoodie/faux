#[faux::create]
struct Foo {
    a: i32,
}

#[faux::methods]
impl Foo {
    pub fn get(&self) -> i32 {
        self.a
    }
}

use faux::when;

#[test]
fn always() {
    let mut foo = Foo::faux();
    when!(foo.get).safe_then(|_| 3);
    for _ in 0..20 {
        assert_eq!(foo.get(), 3);
    }
}

#[test]
fn limited() {
    let mut foo = Foo::faux();
    when!(foo.get).times(3).safe_then(|_| 3);
    for _ in 0..3 {
        assert_eq!(foo.get(), 3);
    }
}

#[test]
#[should_panic]
fn limited_past_limit() {
    let mut foo = Foo::faux();
    when!(foo.get).times(3).safe_then(|_| 3);
    for _ in 0..3 {
        foo.get();
    }
    foo.get(); // panics here
}

#[test]
fn once() {
    let mut foo = Foo::faux();
    when!(foo.get).once().safe_then(|_| 3);
    assert_eq!(foo.get(), 3);
}

#[test]
#[should_panic]
fn once_past_limit() {
    let mut foo = Foo::faux();
    when!(foo.get).once().safe_then(|_| 3);
    foo.get();
    foo.get(); //panics here
}
