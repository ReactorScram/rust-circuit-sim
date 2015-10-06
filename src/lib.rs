#[test]
fn it_works() {
	let world = World::new_half_adder ();
}

// 2 billion junctions is good enough for now
type JunctionIndex = i32;
type ElementIndex = i32;
type OutputIndex = i8;
type Time = i64;
type Level = bool;

enum GateBehavior {
	Inverter,
	And,
	Xor,
}

// In the future, at time "time", junction "junction" will be set to level "level".
pub struct Delay {
	junction: JunctionIndex,
	level: Level,
	time: Time,
}

pub struct Wire {
	input: JunctionIndex,
	output: JunctionIndex,
	delay: Time,
}

pub struct Gate {
	inputs: Vec <JunctionIndex>,
	output: JunctionIndex,
	behavior: GateBehavior,
}

pub struct World {
	wires: Vec <Wire>,
	gates: Vec <Gate>,
	
	junctions: Vec <bool>,
	time: Time,
	delays: Vec <Delay>,
}

impl Wire {
	fn new (input: JunctionIndex, output: JunctionIndex, delay: Time) -> Wire {
		Wire {
			input: input,
			output: output,
			delay: delay,
		}
	}
}

impl World {
	fn new_half_adder () -> World {
		World {
			time: 0,
			delays: vec![],
			junctions: vec![false; 10],
			wires: vec![
				Wire::new (0, 1, 4),
				Wire::new (0, 5, 8),
				Wire::new (2, 3, 4),
				Wire::new (6, 7, 3),
				Wire::new (8, 4, 8),
				Wire::new (8, 9, 4),
			],
			gates: vec![
				Gate {
					inputs: vec![2, 5],
					output: 3,
					behavior: GateBehavior::And,
				},
				Gate {
					inputs: vec![6, 10],
					output: 7,
					behavior: GateBehavior::Xor,
				},
			],
		}
	}
}