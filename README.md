# town-hall
I am making a small and deliberately very simple economics simulation project. The goal is to be able to make this into a cool model that yields interesting statistics
and can easily be translated into something more graphical later to be able to display it more easily. The simulation resembles a purely capitalistic society where
every entity is a random configuration of different weights and biases that decide what its next action will be. Each entity is given all the relevant information about
the world which means that they will all act on diffrent principles. Some might for example see that the resources in the resource pool is running out quickly and
therefore be more conservative with investing whilst another might not care at all about that and care much more about how much money everyone else has and try to
compete with them. Due to the similarities with the real world the results may be insightful.

## village-sim
*village-sim* is the place where the more abstract parts of how the world works is established. It defines the mechanics and flows of how things happen in the world.
It for example defines the abstraction of an enitit and also a village that holds that entity and then runs the simulation.

## neuro-ash
*neuro-ash* is the component that manages the neural networks (tensors) of the entities and runs them on the GPU through *tensor-ash*. It's most important job is to act
as a specialized low-level GPU compute engine that naturally cares about things that make sense for a simulation. It does things such as bundling many entity's networks
together into one big 2D tensor that it can run at once, which is much quicker than running everything sequentially.

# How to run it
To run this project you need to install cargo and rust and then you can simply run this command in the terminal: cargo run --release
