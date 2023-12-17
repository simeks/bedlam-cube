const CUBE_SIZE: usize = 4;
const CUBE_NUM_BITS: usize = CUBE_SIZE * CUBE_SIZE * CUBE_SIZE;
const NUM_PIECES: usize = 13;

enum Axis {
    X,
    Y,
    Z,
}

struct Coords(usize, usize, usize);

fn pack_bit(b: bool, x: usize, y: usize, z: usize) -> u64 {
    (b as u64) << (x * 16 + y * 4 + z)
}
fn unpack_bit(block: u64, x: usize, y: usize, z: usize) -> bool {
    (block >> (x * 16 + y * 4 + z)) & 1 == 1
}

/// Trait for indexing into a block
/// Mainly to support both printing a block as a u64, or as an array of bools
trait BlockIndex<T> {
    fn index(&self, i: T) -> bool;
}

impl BlockIndex<Coords> for u64 {
    fn index(&self, Coords(x, y, z): Coords) -> bool {
        unpack_bit(*self, x, y, z)
    }
}
impl BlockIndex<Coords> for &[[[bool; 4]; 4]; 4] {
    fn index(&self, Coords(x, y, z): Coords) -> bool {
        self[z][y][x]
    }
}

fn print<T>(block: T)
where
    T: BlockIndex<Coords>,
{
    for y in 0..4 {
        for z in 0..4 {
            for x in 0..4 {
                print!(
                    "{}",
                    if block.index(Coords(x, y, z)) {
                        "#"
                    } else {
                        "."
                    }
                );
            }
            print!("    ");
        }
        println!();
    }
}

fn print_solution(picks: &[(usize, u64)]) {
    // Labels for pieces: A, B, C, ...
    let mut arr = [[['0'; 4]; 4]; 4];

    for (piece, placement) in picks.iter() {
        let label = (*piece as u8 + b'A') as char;

        for z in 0..4 {
            for y in 0..4 {
                for x in 0..4 {
                    if placement.index(Coords(x, y, z)) {
                        arr[z][y][x] = label;
                    }
                }
            }
        }
    }

    for z in 0..4 {
        for y in 0..4 {
            for x in 0..4 {
                print!("{}", arr[z][y][x]);
            }
            print!("    ");
        }
        println!();
    }
}

/// Read pieces from file
///
/// File format:
/// 4x4x2 blocks, each piece starting with a piece id (0, 1, 2, ...)
/// z y x: 0123
/// 0 0    0000
/// 0 1    0000
/// 0 2    0000
/// 0 3    0000
/// 1 0    0000
/// 1 1    0000
/// 1 2    0000
/// 1 3    0000
///
/// E.g.:
/// # 0
/// 0100
/// 1110
/// 0100
/// 0000
/// 0000
/// 0000
/// 0000
/// 0000
/// # 1
/// ...
fn read_pieces(path: &str) -> Result<Vec<u64>, std::io::Error> {
    let contents = std::fs::read_to_string(path)?;

    let mut blocks = Vec::new();
    let mut lines = contents.lines();
    loop {
        if lines.next().is_none() {
            break;
        }

        let mut block = 0;
        for z in 0..2 {
            for y in 0..4 {
                let line = lines.next().unwrap();
                for (x, c) in line.chars().enumerate() {
                    if c == '1' {
                        block |= pack_bit(true, x, y, z);
                    }
                }
            }
        }
        blocks.push(block);
    }
    Ok(blocks)
}

/// Rotate piece by 90 degres around the given axis
fn rotate_piece_90(piece: u64, axis: Axis) -> u64 {
    let mut new_piece = 0;
    for z in 0..4 {
        for y in 0..4 {
            for x in 0..4 {
                let (sx, sy, sz) = match axis {
                    Axis::X => (x, 3 - z, y),
                    Axis::Y => (3 - z, y, x),
                    Axis::Z => (3 - y, x, z),
                };
                new_piece |= pack_bit(piece.index(Coords(sx, sy, sz)), x, y, z);
            }
        }
    }
    new_piece
}

/// Translate the piece in the cube by dx, dy, dz
fn translate(piece: u64, dx: i32, dy: i32, dz: i32) -> u64 {
    let mut new_piece = 0;
    for z in 0..4 {
        for y in 0..4 {
            for x in 0..4 {
                let sx = x + dx;
                let sy = y + dy;
                let sz = z + dz;
                if sx < 4 && sy < 4 && sz < 4 && sx >= 0 && sy >= 0 && sz >= 0 {
                    new_piece |= pack_bit(
                        piece.index(Coords(x as usize, y as usize, z as usize)),
                        sx as usize,
                        sy as usize,
                        sz as usize,
                    );
                }
            }
        }
    }
    new_piece
}

/// Generate all unique placements (with all possible rotations and translation) of a piece
fn generate_placements(piece: u64) -> Vec<u64> {
    let mut piece = piece;
    // number of bits in a piece, should always be the same
    // if not, the piece has been shifted outside the cube
    let num_bits = piece.count_ones();

    let mut set = std::collections::HashSet::new();
    for _ in 0..4 {
        for _ in 0..4 {
            for _ in 0..4 {
                piece = rotate_piece_90(piece, Axis::X);
                set.insert(piece);
            }
            piece = rotate_piece_90(piece, Axis::Y);
            set.insert(piece);
        }
        piece = rotate_piece_90(piece, Axis::Z);
        set.insert(piece);
    }
    for piece in set.clone().into_iter() {
        for z in -4..4 {
            for y in -4..4 {
                for x in -4..4 {
                    let piece2 = translate(piece, x, y, z);
                    if piece2.count_ones() == num_bits {
                        set.insert(piece2);
                    }
                }
            }
        }
    }

    set.into_iter().collect()
}

struct Stats {
    num_permutations: usize,
    num_solutions: usize,

    last_print: std::time::Instant,
    last_print_permutations: usize,
}

impl Stats {
    fn new() -> Self {
        Self {
            num_permutations: 0,
            num_solutions: 0,
            last_print: std::time::Instant::now(),
            last_print_permutations: 0,
        }
    }
    fn print(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = (now - self.last_print).as_secs_f64();
        if elapsed < 1.0 {
            return;
        }

        let permutations = self.num_permutations - self.last_print_permutations;
        println!(
            "Permutations: {}, Solutions: {}, Permutations/s: {}",
            self.num_permutations,
            self.num_solutions,
            permutations as f64 / elapsed,
        );
        self.last_print = now;
        self.last_print_permutations = self.num_permutations;
    }
    fn success(&mut self) {
        self.num_solutions += 1;
        self.num_permutations += 1;
    }
    fn fail(&mut self) {
        self.num_permutations += 1;
    }
}

/// Search algorithm
/// state: bit mask of the current state of the cube
/// used_pieces: bit mask of the pieces that have been used
/// bit_map: for each bit in the cube, map it to a list of pieces and piece placement that fit that bit
///         bit_map[bit_index][piece] = [placement0, placement1, ...]
/// picks: stack for keeping track of picked pieces (piece_id, placement)
fn search(
    state: u64,
    used_pieces: u64,
    bit_map: &Vec<Vec<Vec<u64>>>,
    picks: &mut Vec<(usize, u64)>,
    stats: &mut Stats,
    solutions: &mut Vec<Vec<(usize, u64)>>,
) {
    stats.print();
    if used_pieces.count_ones() == NUM_PIECES as u32 {
        // Slows down things quite a lot, but prints each solution
        // print_solution(picks);
        // println!();
        solutions.push(picks.clone());
        stats.success();
        return;
    }

    let bit_index = state.trailing_ones() as usize;

    // For each piece that fits this bit, recurse
    for piece in 0..NUM_PIECES {
        if used_pieces & (1 << piece) != 0 {
            continue;
        }
        for permutation in bit_map[bit_index][piece].iter() {
            if (*permutation & state) == 0 {
                picks.push((piece, *permutation));
                search(
                    state | *permutation,
                    used_pieces | 1 << piece,
                    bit_map,
                    picks,
                    stats,
                    solutions,
                );
                picks.pop();
            }
        }
    }
    stats.fail();
}

fn main() {
    let pieces = read_pieces("pieces.txt").expect("Failed to read pieces");
    for (piece, piece_bits) in pieces.iter().enumerate() {
        println!("Piece {}", piece);
        print(*piece_bits);
        println!();
    }

    println!("Read {} pieces", pieces.len());
    println!();
    if pieces.len() != NUM_PIECES {
        panic!("Expected {} pieces, got {}", NUM_PIECES, pieces.len());
    }

    let piece_placements = pieces
        .into_iter()
        .map(generate_placements)
        .collect::<Vec<_>>();

    for (piece, placements) in piece_placements.iter().enumerate() {
        println!("Piece {}: {} permutations", piece, placements.len());
    }
    println!();

    // For every bit in the block, map it to a each piece and permutation
    let mut bit_map: Vec<Vec<Vec<u64>>> = vec![vec![Vec::new(); NUM_PIECES]; CUBE_NUM_BITS];
    for bi in 0..CUBE_NUM_BITS {
        for pi in 0..NUM_PIECES {
            let map_placement = &mut bit_map[bi][pi];
            for placement in piece_placements[pi].iter() {
                if placement & (1 << bi) != 0 {
                    map_placement.push(*placement);
                }
            }
        }
    }

    let mut stats = Stats::new();
    // Stack for keeping track of picked pieces
    let mut picks = vec![(0, 0); NUM_PIECES];
    let mut solutions = Vec::new();
    search(0, 0, &bit_map, &mut picks, &mut stats, &mut solutions);

    println!("Found {} solutions", solutions.len());
    println!();
    for (i, solution) in solutions.iter().enumerate() {
        println!("Solution {}", i);
        print_solution(solution);
        println!();
    }
}
