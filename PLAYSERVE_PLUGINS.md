DBus API

DBus provides a very nice interface for inter-process communication and is a common standard for desktop Linux. Plugins can be written in any language, and it allows communication via unix sockets, so it is fast, and does not carry the overhead of TCP.

It also has useful built-in design patterns:

Method calls

Read-only or read/write properties; including signals when properties change

Signals; to notify listeners about something

Refactor to Provider Enum

As of this moment, Playserve relies of a static enum definition for Providers. And if we want to support different plugins, we should make that a simple string instead of an enum so we can support dynamic providers.

Reference

item_id: The ID of an app in a provider’s store.

Interfaces

one.playtron.plugin.Manager

Methods

RegisterPlugin(bus_name: String, plugin_path: ObjectPath)

Description: registers the plugin with the plugin manager

Arguments:

bus_name: String 

name of the bus on DBus that the plugin is listening on (e.g. one.playtron.plugin.Steam)

plugin_path: ObjectPath 

path on DBus that the plugin is listening on (e.g. /one/playtron/plugins/Steam)

Example Usage:

busctl --user call one.playtron.Playserve \
  /one/playtron/plugins/Manager \
  one.playtron.plugin.Manager \
  RegisterPlugin "so" "one.playtron.SteamBus" "/one/playtron/SteamBus/SteamClient0"



IsPluginRegistered(plugin_id: String) → bool

Description: returns whether a certain plugin is registered

Arguments:

plugin_id: String

unique plugin id of the plugin (e.g. “steam”)

busctl --user call one.playtron.Playserve \
  /one/playtron/plugins/Manager \
  one.playtron.plugin.Manager \
  IsPluginRegistered "s" "steam"



Properties

Version: String

Description: version of the plugin manager (e.g. “0.1.0”)

Example Usage:

busctl --user get-property one.playtron.Playserve \
  /one/playtron/plugins/Manager \
  one.playtron.plugin.Manager \
  Version



Signals

PluginRegistered(plugin_id: String, plugin_type: PluginType)

Description: Emitted when a new plugin has been registered



OnDriveAdded(vendor: String, model: String, hint_name: String, name: String, available_space: u64, max_size: u64, path: String, file_system: String, is_root: bool, needs_formatting: bool)

Description: Emitted when a new drive is connected



OnDriveRemoved(drive_name: String)

Description: Emitted when a drive is disconnected



one.playtron.plugin.Plugin

Properties

Id: String 

Description: unique plugin id of the plugin (e.g. “steam”)

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.Plugin \
  Id



Name: String 

Description: Human readable name of the plugin (e.g. “Steam”)

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.Plugin \
  NameInstalledAppDescription



Version: String 

Description: version of the plugin (e.g. “0.1.0”)

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.Plugin \
  Version



MinimumApiVersion: String 

Description: the plugin API version that the plugin supports

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.Plugin \
  MinimumApiVersion 



one.playtron.plugin.LibraryProvider

Interface for defining a library provider plugin.

Methods

GetInstallOptions(app_id: String) → []InstallOptionDescription

Description: Returns a list of available install options for the given provider app id. Install options are a list of arbitrary values that the library provider knows about which can affect what version of the app is installed and how it is installed (e.g. “version”, “language”, “low-violence”, “branch”, etc.)

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  GetInstallOptions "s" "460950"

GetLaunchOptions(app_id: String) → []LaunchOption

Description: Returns a list of available launch options. This includes information on how the game should be launched together with other useful metadata.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  GetLaunchOptions "s" "460950"

Install(app_id: String, disk_path: String, options: Dictionary)

Description: Start installing the app with the given provider app id to the given block device. It is the plugin’s responsibility to determine where on the mounted disk the app will be installed. Available install options can be queried by calling the GetInstallOptions method. Once the install has started, the plugin will be responsible for sending InstallProgressed signals about the progress of the install.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  Install "ssa{sv}" "460950" "/dev/sda1" 2 'os' s windows 'language' s english



Uninstall(app_id: String)

Description: Uninstalls the given app.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  Uninstall "s" "460950"



MoveItem(app_id: String, string disk) → String

Description: Moves the given app to another disk and returns the new install directory.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  MoveItem "ss" "460950" "/dev/sda3"



Update(app_id: String)

Description: Start updating the given game. Once the update starts, the plugin will be responsible for sending signals about the progress of the update.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  Update "s" "460950"



GetAppMetadata(app_id: String) → ItemMetadata

Description: Returns the metadata of the app.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  GetAppMetadata "s" "460950"



GetProviderItem(app_id: String) -> []ProviderItem

Description: Returns all provider items.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  GetProviderItem "s" "460950"



GetProviderItems() -> []ProviderItem

Description: Returns all provider items.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  GetProviderItems



Refresh()

Description: Trigger discovery of provider items. Should emit LibraryUpdated signal if new items are discovered.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  Refresh


GetSavePathPatterns(app_id: String, platform: String) → []CloudPath

Description: Obtain the list of CloudPaths applicable for given app_id and platform, if the list is empty it is assumed saves are not supported

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  GetSavePathPatterns ss 730 windows



Properties

Provider: String

Description: Name of the provider which this plugins represents (has to be unique between all library provider plugins)

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  Provider



Name: String

Description: Human readable name of the library provider

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.LibraryProvider \
  Name



Signals

InstallStarted(app_id: String, version: String, install_directory: String, total_download_size: u64, requires_internet_connection: bool)

Description: Emitted when an install starts

Args:

    item_id: identifier of the game that was installing

    status: status of the update process.



InstallProgressed(app_id: String, stage: uint, downloaded_bytes: u64, total_download_size: u64, progress: f32)

Description: Emitted when a game’s installation step is in progress.



InstallCompleted(app_id: String)

Description: Emitted when a game’s installation step is completed.



InstallFailed(app_id: String, error: String)

Description: Emitted when a game’s installation step is failed.



MoveItemProgressed(app_id: String, progress: f32)

Description: Emitted during MoveItem, sending progress of the move.



AppNewVersionFound(app_id: String, version: String)

Description: Emitted when a installed game has a new version available to update.



LibraryUpdated(new_items: []ProviderItem)

Description: Emitted when provider received an event about new library entries

Args: 
    new_items: Contains new/updated library items



InstalledAppsUpdated()

Description: Emitted when installed apps store has been updated.

one.playtron.plugin.CloudSaveProvider

Interface for defining how to synchronize cloud saves.

Methods

CloudSaveUpload(app_id: String, platform: String, force: bool, path_patterns: []CloudPath)

Description: Start uploading the save data for the given provider app id using the given path patterns.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.CloudSaveProvider \
  CloudSaveUpload "ssba(sssbas)" "460950" 1 "/path/to/saves"



CloudSaveDownload(app_id: String, platform: String, force: bool, path_patterns: []CloudPath)

Description: Start downloading the save data for the given provider app id using the given path patterns.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.plugin.CloudSaveProvider \
  CloudSaveDownload "ssba(sssbas)" "460950" 1 "/path/to/saves"



Signals

CloudSaveSyncProgressed(app_id: String, progress: f64, state: CloudSyncOperation)

CloudSaveSyncFailed(app_id: String, error: String, local_update_time: u64, remote_update_time: u64, quota_usage: u64, quota: u64)

one.playtron.auth.Cryptography

Interface for encrypting secrets being sent over the dbus user session to prevent eavesdropping.

Methods

GetPublicKey() → PublicKey

Description: Returns the public key used to send encrypted secrets. The data must be returned in a PEM encoded string. The key type must be one of: ["RSA-SHA256"].

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.auth.Cryptography \
  GetPublicKey

one.playtron.auth.User

Interface exposing authorization state and applicable user details.

Methods

ChangeUser(user_id: String) → bool

Description: Switch session to chosen user. Called after initialization of the plugin is done and session is available. Also used for account switching. Returns whether it was possible to re-use that session.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.auth.User \
  ChangeUser "s" "PlaytronTest1"

Logout(user_id: String)

Description: Logs specified user out of the provider.

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.auth.User \
  Logout "s" "PlaytronTest1"

Properties

Status: Enum ProviderStatus

Description: The status of the provider authentication

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.auth.User \
  Status

Username: String

Description: The displayname/username of the currently authenticated user. This will be empty if none are authenticated. Empty when no user is authenticated

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.auth.User \
  Username

Identifier: String

Description: Internal identifier for given user. Value provided here shouldn’t be mutable. E.g usernames that can be changed shouldn’t be used here. Empty when no user is authenticated

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.auth.User \
  Identifier

Avatar: String

Description: An url to the avatar image. Empty if no avatar is available.

Example Usage:

busctl --user get-property one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.auth.User \
  Avatar

one.playtron.auth.PasswordFlow

Interface for defining how to log in to a provider using simple username and password authentication.

Methods

Login(username: String, password: String)

Description: Log in to the provider with the given username and password

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.auth.PasswordFlow \
  Login "ss" "PlaytronTest1" "mysecretpassword"

one.playtron.auth.TwoFactorFlow

Interface for defining how to log in to a provider using a two-factor code.

Methods

SendCode(code: String)

Description: Send the given two-factor code

Example Usage:

busctl --user call one.playtron.SteamBus \
  /one/playtron/SteamBus/SteamClient0 \
  one.playtron.auth.TwoFactorFlow \
  SendCode "s" "FY3LK"

Signals

TwoFactorRequired(previous_code_incorrect: bool, message: String)

Description: Emitted when two-factor authentication is required in order to log in.

Args:

previous_code_incorrect: will be true if an incorrect code has been sent before

message: message to display to the user



EmailTwoFactorRequired(email: String, previous_code_incorrect: bool, message: String)

Description: Emitted when email 2FA is required in order to log in.

Args:

email: the email address where the 2FA code was sent

previous_code_incorrect: will be true if an incorrect code has been sent before

message: message to display to the user

ConfirmationRequired(message: String)

Description: Emitted when auth confirmation is required. Whether through email link or mobile app confirmation.

Args:

message: message to display to the user

one.playtron.auth.PortalFlow

Interface for defining how to log in to a provider using a portal flow where a webview is opened to get an authorization token.

Methods

Login(url: String, extra: String)

Description: Finish the flow with given final URL and extra data obtained with JS code (empty if no script was defined).

Properties

Url: String

Description: The url used to initiate the auth flow.

IsFinishedScript: String

Description: JavaScript boolean expression determining if the flow is finished 

OnFinishScript: String

Description: JavaScript code that will run after final url is reached. Returned data will be sent over as an extra parameter for login. If empty, no extra data will be obtained

AllowedRoutes: []String

Description: A list of routes that are allowed for the flow e.g https://auth.gog.com. Note if any route that’s not on the list is reached the flow will be cancelled resulting in auth error.

PopupUrlSuffix: String

Description: This is used to determine whether the url should be opened in the separate webview, useful for OAuth2 flows that open in separate tab/window.

ShowLibrarySyncView: bool

Description: Whether the UI should navigate to the view displaying the import progress of the games. This should be true unless a plugin doesn’t implement LibraryProvider

one.playtron.auth.QrFlow

Interface for defining how to log in to a provider using a QR Code

Methods

Begin()

Description: Gets the code to encode in the QR image. Immediately emits a CodeUpdated signal

Cancel()

Description: Stops updates of new QR codes. Use in an event of cancellation of the auth flow.

Signals

CodeUpdated(token: String)

Description: Emited every time a new QR code needs to be displayed. For example when the previous one is no longer valid

one.playtron.auth.ApiToken

Interface for defining how to log in to a provider using an API Token

Methods

Login(api_token: String) → String

Description: Logs the user in using an API token

one.playtron.plugin.ArtworkProvider

Interface for defining an artwork provider plugin. This could be used to show custom artwork for games in the library, such as art from SteamGridDB.

Methods

GetArtworkMetadata(item_id: String) → []ArtworkMetadata

Description: Returns a list of ArtworkMetadata.

one.playtron.plugin.Settings

Interface for defining an artwork provider plugin. This could be used to show custom artwork for games in the library, such as art from SteamGridDB.

Methods

GetArtworkMetadata(item_id: String) → []ArtworkMetadata

Description: Returns a list of ArtworkMetadata.

one.playtron.CompatibilityTool

Interface to provide plugins with needed functionality for compatibility tools

Methods

GetWinePrefix(item_id: String, provider: Provider) → String

Description: gets the wine prefix path of an item



GetCompatibilityTools() → []CompatibilityTool

Description: gets a list of installed compatibility tools

one.playtron.plugin.CompatibilityTool

Interface for defining how to handle compatibility tools for games

Methods

GetLaunchMetadata(item_id: String, install_path: String, default_executable_path: String) → LaunchMetadata

Description: Gets the metadata needed to launch an item



Properties

name: String

Description: Name of the compatibility tool



version: String

Description: Version of the compatibility tool

url: String

Description: Url where the tool can be downloaded from



executable: String

Description: Executable used to launch the tool



one.playtron.plugin.SubscriptionProvider

Interface for defining interaction with subscriptions like EA Play, Ubisoft+, Game Pass.

Methods

GetApps() → []String

Description: Returns a list of applications that were added to the library only thanks to the subscription.

Properties

Name: String

Description: Name of an subscription

ManagementUrl: String

Description: Url to a website where management of a subscription happens.

Enums

PluginType

LibraryProvider = 0, This plugin type will focus its functionality on provider methods for the user to authenticate with a certain provider, fetch their library of apps, and launch their apps.

ArtworkProvider = 1, This plugin type will provider custom art work for different games based on their respective provider ids.

AppType

Game = 0

Application = 1

Tool = 2

Dlc = 3

Music = 4

Config = 5

Demo = 6

Beta = 7

UpdateStage

None = 0

Error = 1

Preallocating = 2

Downloading = 3

Verifying = 4

Installing = 5

Done = 6

PlayserveStatus

PL_SUCCESS = 0, Code when there is no issue

PL_GENERIC = 1, Returned in case the error could not be identified

PL_FILE_PERMISSION = 2, Returned if there has been an error associated with file permission

PL_NO_NETWORK = 3, Returned if the system has no network connection when it needs it

PL_NOT_ENOUGH_SPACE = 4, Returned if there is not enough space available in the disk for a download

PL_NOT_AUTHENTICATED = 5, Returned if the user needed authentication but none was found or an error happened related to it

ProviderStatus

Unauthorized = 0

Requires2fa = 1

Authorized = 2

ToolType

Unknown = 0

Wine = 1

Proton = 2

LaunchType

Unknown = 0

Launcher = 1

Game = 2

Tool = 3

Document = 4

Other = 5

Structs

InstallOptionDescription

id: String

Identifier for the install option. This is used to provide a common library-agnostic identifier for common install options.

“language”: the language of the game to install

“branch”: the branch of the game to install

“version”: the version of the game to install

“os”: the os platform of the game to install

“architecture”: the cpu architecture of the game to install

name: String

The name of the provider-specific install option. (E.g. “version”, “os”, “language”, “low-violence”)

human_readable_name: String

A human-readable string of the install option appropriate for showing in the UI. (E.g. “Version”, “Operating System”, etc.)

values: []String

Possible values that can be passed for this install option. For example, if this option was for the version of the game to install, this list could be a list of versions of the game that are available to be installed (e.g. [“v2.1”, “v2.0”, “v1.5”, “v1.3”]) 

LaunchOption

description: String

The name of the option. May be an empty string when the option isn’t any special.

executable: String

Path to executable that should be launched. Usually relative to working directory. However it can also be set as absolute

arguments: String

Arguments that are to be provided to the executable

working_directory: String

Absolute path to working directory from which the game has to be started.

environment: [](String, String)

Array of Key - Value tuples. That describe additional environment variables that need to be set.

launch_type: LaunchType

If known, this describes what the target actually is. Used for presentation to the user as well as potentially for skipping game launchers if possible.

hardware_tags: []String

List of tags that apply to this option. Tags point that the action is preferred when running on particular piece of hardware. Currently defined hardware tags:

 steamdeck - when on steam deck the launch option will be used instead of the default

ItemMetadata

name: String

install_size: u64

requires_internet_connection: bool

cloud_save_folders: String[]

installed_version: String

latest_version: String

install_size, installed_version, and latest_version will all be empty strings if the item is not installed.

ProviderItem

id: String

name: String

can be set to empty string and app_type to just Game and rely on the value returned from the Playtron API instead because some providers like GOG are not easy to fetch all the game info at once

provider: String

app_type: AppType

ArtworkImage

url: String

type: String

ArtworkMetadata

item_id: String

provider: String

images: []ArtworkImage

CompatibilityTool

id: i32

slug: String

description: String

tool_type: ToolType

install_path: String

LaunchMetadata

executable: String

arguments: []String

environment: Map<String, String>

PublicKey

key_type: String

the type of key. Can only currently be: RSA-SHA256

key_data: String

the PEM encoded string of the public key

CloudPath

alias: String

interal identifier of a cloud save path, usually a name used to determine a container on the server

path: String

file system location, when coming from LibraryProvider can be a windows path, before coming to CloudSaveProvider it will get translated to Linux location

pattern: String

glob pattern used to check against when collecting files for upload

recursive: bool

whether to recursively scan the subdirectories of the path

platforms: []String

platforms this cloud location is limited to (if applicable)
