TODO:
- handle disconnects (in any order, for both tcp and udp)
- unit testing

Known bugs:
- apparently, at least on gnome, using multiple desktops fucks up the speed at which sockets receive information?
	- can be verified as follows
		- use multiple desktop spaces
		- run the game
		- notice that the game lags
		- press the system key
		- notice that it doesnt
		- move everything to a single desktop space
		- notice that everything is fine again
