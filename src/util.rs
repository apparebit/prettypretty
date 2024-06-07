/// A safe, symbolic index for the three color coordinates.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Coordinate {
    C1 = 0,
    C2 = 1,
    C3 = 2,
}
