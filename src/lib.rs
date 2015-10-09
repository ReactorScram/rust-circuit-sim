use std::cmp;

// 2 billion junctions is good enough for now
pub type JunctionIndex = usize;
pub type Time = i64;
pub type Level = bool;

pub enum GateBehavior {
	And,
	Not,
	Or,
	Xor,
}

// In the future, at time "time", junction "junction" will be set to level "level".
#[derive (Clone, Copy)]
pub struct Signal {
	junction: JunctionIndex,
	level: Level,
	time: Time,
}

#[derive (Clone, Copy)]
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
	// These don't change at runtime
	wires: Vec <Wire>,
	gates: Vec <Gate>,
	
	junctions: Vec <Level>,
	time: Time,
	signals: Vec <Signal>,
}

pub enum WorldCreationErr {
	// Two elements point to the same junction, this is illegal
	FanIn,
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
	pub fn new (wire_tuples: Vec <(JunctionIndex, JunctionIndex, Time)>, gates: Vec <Gate>) -> Result <World, WorldCreationErr> {
		let wires: Vec <Wire> = wire_tuples.iter ().map (|tuple| Wire::new (tuple.0, tuple.1, tuple.2)).collect ();
		
		let max_junction_wires = wires.iter ().fold (0, |max, wire| cmp::max (max, cmp::max (wire.input, wire.output)));
		let max_junction_gates = gates.iter ().fold (0, |max, gate| {
			let max_input = gate.inputs.iter ().max ();
			
			let max_junction_gate = match max_input {
				Option::Some (junction) => cmp::max (*junction, gate.output),
				Option::None => gate.output,
			};
			
			cmp::max (max, max_junction_gate)
		});
		
		let max_junction = cmp::max (max_junction_gates, max_junction_wires);
		let junction_count = max_junction + 1;
		
		let mut junction_has_input = vec! [false; junction_count];
		
		for wire in wires.iter () {
			if junction_has_input [wire.output] {
				return Err (WorldCreationErr::FanIn)
			}
			
			junction_has_input [wire.output] = true;
		}
		
		for gate in gates.iter () {
			if junction_has_input [gate.output] {
				return Err (WorldCreationErr::FanIn);
			}
			
			junction_has_input [gate.output] = true;
		}
		
		Ok (World {
			time: 0,
			signals: vec![],
			junctions: vec![false; junction_count],
			wires: wires,
			gates: gates,
		})
	}
	
	pub fn new_half_adder () -> World {
		World::new (
		vec![
			(0, 1, 4),
			(0, 5, 8),
			(2, 3, 4),
			(6, 7, 3),
			(8, 4, 8),
			(8, 9, 4),
		],
		vec![
			Gate {
				inputs: vec![1, 4],
				output: 2,
				behavior: GateBehavior::And,
			},
			Gate {
				inputs: vec![5, 9],
				output: 6,
				behavior: GateBehavior::Xor,
			},
		]).ok ().expect ("Half adder circuit is invalid")
	}
	
	pub fn new_full_adder () -> World {
		World::new (
		vec![
			(0, 3, 1),
			(0, 5, 1),
			(1, 4, 1),
			(1, 6, 1),
			(2, 11, 1),
			(2, 9, 1),
			(7, 10, 1),
			(7, 8, 1),
			(12, 14, 1),
			(13, 15, 1),
		],
		vec![
			Gate {
				inputs: vec![3, 4],
				output: 7,
				behavior: GateBehavior::Xor,
			},
			Gate {
				inputs: vec![10, 11],
				output: 16,
				behavior: GateBehavior::Xor,
			},
			Gate {
				inputs: vec![8, 9],
				output: 12,
				behavior: GateBehavior::And,
			},
			Gate {
				inputs: vec![5, 6],
				output: 13,
				behavior: GateBehavior::And,
			},
			Gate {
				inputs: vec![14, 15],
				output: 17,
				behavior: GateBehavior::Or,
			},
		]).ok ().expect ("Full adder circuit is invalid")
	}
	
	pub fn is_settled (& self) -> bool {
		self.signals.len () == 0
	}
	
	fn sort_signals (&mut self) {
		// This step is just a safety since we should be insertion sorting the delays already
		self.signals.sort_by (|a, b| a.time.cmp (&b.time));
	}
	
	fn step_gates (&mut self) {
		let mut new_signals = Vec::<Signal>::new ();
		
		// TODO: Optimize to only touch gates whose inputs have changed
		for gate in self.gates.iter () {
			let inputs: Vec <bool> = gate.inputs.iter ().map (|i| self.junctions [*i]).collect ();
			
			let output = match gate.behavior {
				GateBehavior::And => inputs.iter ().fold (true, |sum, a| sum && *a),
				GateBehavior::Not => ! inputs [0],
				GateBehavior::Or => inputs.iter ().fold (false, |sum, a| sum || *a),
				GateBehavior::Xor => inputs.iter ().fold (false, |sum, a| sum ^ *a),
			};
			
			let destiny_level = self.get_junction_destiny (gate.output);
			
			if destiny_level != output {
				new_signals.push (Signal {
					junction: gate.output,
					time: self.time,
					level: output,
				});
			}
		};
		
		// TODO: Proper insertion sorting
		// If the number of new signals is small compared to the number of 
		// in-flight signals, I could do a few binary insertions sorts.
		// If the number is large, it might be better to do a mergesort.
		for signal in new_signals {
			self.signals.push (signal);
		}
		
		self.sort_signals ();
	}
	
	fn get_junction_destiny (& self, junction: JunctionIndex) -> Level {
		if self.signals.len () > 0 {
			// Find the last signal that was heading for that junction
			// Remember that junctions are not allowed to fan in
			for i in 0 .. self.signals.len () {
				let signals = self.signals [self.signals.len () - i - 1];
				
				if signals.junction == junction {
					return signals.level;
				}
			}
		}
		
		self.junctions [junction]
	}
	
	fn step_wires (&mut self) {
		for wire in self.wires.iter () {
			let input = self.junctions [wire.input];
			
			// Don't push redundant delays
			let destiny_level = self.get_junction_destiny (wire.output);
			
			if input != destiny_level {
				self.signals.push (Signal {
					junction: wire.output,
					time: self.time + wire.delay,
					level: input,
				});
			}
		};
		
		// TODO: Proper insertion sorting
		self.sort_signals ();
	}
	
	fn propagate_signals (&mut self) {
		let next_time = self.signals [0].time;
		
		for signal in self.signals.iter () {
			if signal.time == next_time {
				self.junctions [signal.junction] = signal.level;
			}
			else if signal.time > next_time {
				break;
			}
		}
		
		self.signals.retain (|signal| signal.time > next_time);
		
		self.time = next_time;
	}
	
	pub fn set_junction (&mut self, junction: JunctionIndex, level: Level) {
		self.signals.push (Signal {
			junction: junction,
			level: level,
			time: self.time,
		});
	}
	
	pub fn step (&mut self) {
		if self.is_settled () {
			return;
		}
		
		self.sort_signals ();
		
		self.propagate_signals ();
		
		self.step_gates ();
		self.step_wires ();
	}
	
	pub fn step_to_settled (&mut self) {
		while ! self.is_settled () {
			self.step ();
		}
	}
}

#[test]
pub fn test_fan_in () {
	let world_or_err = World::new (
		vec! [
		(0, 2, 1),
		(1, 2, 1),
		],
		vec! []
	);
	
	if let Err (WorldCreationErr::FanIn) = world_or_err {
		// Good
	}
	else {
		panic! ("World creation should have thrown a FanIn error");
	}
}

#[test]
pub fn test_half_adder () {
	let mut world = World::new_half_adder ();
	world.step_to_settled ();
	
	let assert_outputs = |world: &World, msb: Level, lsb: Level| {
		assert_eq! (world.junctions [3], msb);
		assert_eq! (world.junctions [7], lsb);
	};
	
	assert_outputs (&world, false, false);
	
	// Junctions 0 and 8 are the input bits
	
	world.set_junction (0, true);
	world.step_to_settled ();
	assert_outputs (&world, false, true);
	
	world.set_junction (8, true);
	world.step_to_settled ();
	assert_outputs (&world, true, false);
	
	world.set_junction (0, false);
	world.step_to_settled ();
	assert_outputs (&world, false, true);
}

#[test]
pub fn test_full_adder () {
	let mut world = World::new_full_adder ();
	world.step_to_settled ();
	
	let assert_outputs = |world: &mut World, a: Level, b: Level, c: Level, carry: Level, sum: Level| {
		world.set_junction (0, a);
		world.set_junction (1, b);
		world.set_junction (2, c);
		
		world.step_to_settled ();
		
		assert_eq! (world.junctions [17], carry);
		assert_eq! (world.junctions [16], sum);
	};
	
	// Truth table for a full adder
	assert_outputs (&mut world, false, false, false, false, false);
	assert_outputs (&mut world, false, false, true, false, true);
	assert_outputs (&mut world, false, true, true, true, false);
	assert_outputs (&mut world, false, true, false, false, true);
	
	assert_outputs (&mut world, true, true, false, true, false);
	assert_outputs (&mut world, true, true, true, true, true);
	assert_outputs (&mut world, true, false, true, true, false);
	assert_outputs (&mut world, true, false, false, false, true);
}
