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
pub struct Delay {
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

enum JunctionDestiny {
	Settled (Level),
	Eventually (Delay),
}

impl World {
	pub fn new (wire_tuples: Vec <(JunctionIndex, JunctionIndex, Time)>, gates: Vec <Gate>) -> World {
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
		
		World {
			time: 0,
			delays: vec![],
			junctions: vec![false; junction_count],
			wires: wires,
			gates: gates,
		}
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
		])
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
		])
	}
	
	pub fn is_settled (& self) -> bool {
		self.delays.len () == 0
	}
	
	fn sort_delays (&mut self) {
		// This step is just a safety since we should be insertion sorting the delays already
		self.delays.sort_by (|a, b| a.time.cmp (&b.time));
	}
	
	fn step_gates (&mut self) {
		let mut new_delays = Vec::<Delay>::new ();
		
		// TODO: Optimize to only touch gates whose inputs have changed
		for gate in self.gates.iter () {
			let inputs: Vec <bool> = gate.inputs.iter ().map (|i| self.junctions [*i]).collect ();
			
			let output = match gate.behavior {
				GateBehavior::And => inputs.iter ().fold (true, |sum, a| sum && *a),
				GateBehavior::Not => ! inputs [0],
				GateBehavior::Or => inputs.iter ().fold (false, |sum, a| sum || *a),
				GateBehavior::Xor => inputs.iter ().fold (false, |sum, a| sum ^ *a),
			};
			
			let destiny = self.get_junction_destiny (gate.output);
			let destiny_level = match destiny {
				JunctionDestiny::Settled (level) => level,
				JunctionDestiny::Eventually (delay) => delay.level,
			};
			
			if destiny_level != output {
				new_delays.push (Delay {
					junction: gate.output,
					time: self.time,
					level: output,
				});
				
				println! ("Gate {} set to {}", gate.output, output);
			}
		};
		
		for delay in new_delays {
			self.delays.push (delay);
		}
		
		// TODO: Proper insertion sorting
		self.sort_delays ();
	}
	
	fn get_junction_destiny (& self, junction: JunctionIndex) -> JunctionDestiny {
		if self.delays.len () > 0 {
			//println! ("Delays:");
			
			for i in 0 .. self.delays.len () {
				let delay: Delay = self.delays [self.delays.len () - i - 1];
				
				//println! ("At {} junction {} will be {}", delay.time, delay.junction, delay.level);
				
				if delay.junction == junction {
					return JunctionDestiny::Eventually (delay);
				}
			}
		}
		
		JunctionDestiny::Settled (self.junctions [junction])
	}
	
	fn step_wires (&mut self) {
		for wire in self.wires.iter () {
			let input = self.junctions [wire.input];
			
			// Don't push redundant delays
			let destiny = self.get_junction_destiny (wire.output);
			let destiny_level = match destiny {
				JunctionDestiny::Settled (level) => level,
				JunctionDestiny::Eventually (delay) => delay.level,
			};
			
			if input != destiny_level {
				self.delays.push (Delay {
					junction: wire.output,
					time: self.time + wire.delay,
					level: input,
				});
			}
		};
		
		// TODO: Proper insertion sorting
		self.sort_delays ();
	}
	
	fn propagate_delays (&mut self) {
		let next_time = self.delays [0].time;
		
		for delay in self.delays.iter () {
			if delay.time == next_time {
				self.junctions [delay.junction] = delay.level;
				
				println! ("Junction {} set to {}", delay.junction, delay.level);
			}
		}
		
		self.delays.retain (|delay| delay.time > next_time);
		
		self.time = next_time;
	}
	
	pub fn set_junction (&mut self, junction: JunctionIndex, level: Level) {
		self.delays.push (Delay {
			junction: junction,
			level: level,
			time: self.time,
		});
	}
	
	pub fn step (&mut self) {
		if self.is_settled () {
			return;
		}
		
		self.sort_delays ();
		
		self.propagate_delays ();
		
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
