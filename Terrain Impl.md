
Currently, there are two crates
- Marching Cubes Terrain 
- Voxel Terrain

This impl overall is *rough...* to say the least
For one, both utilize the same noise from `noiz` but both have their *own* impl of that

Other solutions:
Solution 1: Terrain crate with
  - noise
  - marching cubes
  - voxel
  - high level manager
  
  tight knit and probably easy to implement

Solution 2: World crate with noise
  - marching cubes high level manager
  - voxel high level manager
  - noise terrain applyer for each
  
  more modular and probably more flexible with potential future designs
  Although likely much more difficult to implement and potentially pretty ineffecient 
  as there would have to be a layer of abstraction between the terrain and the physicality of the terrain

Solution 3: Keep as is with potentail added noiz wrapper crate with terrain specifics and random help
  - noise
  - marching cubes terrain
  - voxel terrain
  
  I'd probably repeat myself a lot with this implementation and the logic would vary greatly making it hard to 
  upkeep

Solution 4: A build off of 2, althuogh the world has low level bindins to terrain impl
- Marching cubes which can be sperated from terrain but has low level impl of terrain
- Same but voxel ^
- High level terrain noiz both can use

This is probably the best, requires the most refactoring of the current code
