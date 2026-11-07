
create an app to automate valorant theming

1. locate \VALORANT\live\ShooterGame\Content\Movies\Menu

2. find an mp4 file named Homescreen.mp4, may have a pre or postfix eg: 13_06_Homescreen.mp4

3. keep a backup of the file and replace it with another .mp4 file under the same name.

this replaces the background in valorant, but when the game launches riotgames will check if the file is different and download an update to replace it, so the file must be replaced after the riot client closes. this can be done manually but i want to automate it. additionally, after valorant closes the video must be replaced with the original to avoid the update when the game is launched normally.

nothing should be injected or overlayed into the game, this is just a utility.

the app should be a system tray first app that has a modern animated ui designed for desktop with ux in mind. the tray should have various options eg launch game, change video, settings etc. but have a sleek modern ui for the user to use aswell. the pre and post fixes aswell as the whole filename could change, and there could be multiple versions of the same filename with the prefix changed, so have a smart detection system and an override for the user to select a specific file to run the scan with. there should be an option to change theme color and the app should have a light and dark theme aswell.

the app should ask for admin when needed and have an opt in option to always have custom video even when launching from outside the app (running in background) and an op in option to start with windows

i was thinking of using dearimgui or tauri for the app, use askuserquestion with any other suggestions if you have any for decisions throughout the project.

use a cool icon and give it a one word sleek/cool name aswell.

the app should have an installer aswell.
i want to upload this to github with releases and a pretty and informative readme.md

also link to https://github.com/ChirisaCasey/Val-Patcher as a lite .bat version of the app that gives users easy access to see what the program does, if you want only after building the app, you can clone this one and improve it.