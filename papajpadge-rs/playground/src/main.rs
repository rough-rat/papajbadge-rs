use abc::{AbcIter, Event};

fn main() {
    let abc = b"
X:1
T:Barka
M:6/8
L:1/8
R:jig
K:C
C6 | A3 A3- | AAB cBA | G3 G3- | G3 F2 E | F3 F3- | FFG AGF | E3 E3- |
 E3 C2 C | A3 A3- | AAB cBA | G3 G3- | G3 F2 E | F3 F3- | FDE FED | C6 | x6 |
E6- | EDE FED | C3 C3- | C3 D2 E    | F3 F3-  | F2 F FFE | D3 D3- | D2 G, C2 D |
E3 E3-  | E2 E F2 D | C3 C3- | C6
";

    let iter = AbcIter::new(abc, 320).expect("valid header");

    for ev in iter {
        println!("{:?}", ev);
    }
}

