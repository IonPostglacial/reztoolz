use std::path::Path;

struct Point {
    x: u32,
    y: u32,
}

struct Background<'a> {
    tiles: &'a [u8],
    width: u32,
    height: u32,
}

enum LogicKind {}

struct Logic<'a> {
    kind: LogicKind,
	graphic: &'a Path,
	animation: &'a Path,
	data: &'a [u8],
}
 
struct World<'a> {
    id: u32,
	area: u32,
	width: u32,
	height: u32,
	start: Point,
	background: Option<Background<'a>>,
	tiles: &'a [u8],
	logics: Vec<Logic<'a>>,
}