This is a marker for the desired structure of the "levels"

Levels are Stacked, basic_walls, layout and spawn##.ron

I.e 
basic_walls + test_layout/layout.ron + spawn00.ron

The basic walls should set the master bounds

The layout folder and layout.ron specify the additional fixed boundaries. I.e. what makes this 
level unique, I.e. walls, shapes, blockades etc.

The spawn00.ron places the spawn points and other widgets in a layout, I can re-use a layout for multiple placements.

the root levels.ron should track the registry of levels
