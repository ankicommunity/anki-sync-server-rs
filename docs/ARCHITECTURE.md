This page illustrates how the server is constructed。
It consists of two parts，one is about collection sync，the other is about media sync。
The server uses crate `actix-web` as web frame work and thus uses `async` feature.
So the entry point of the server is function `main` from `main.rs`.Here's the feature named `tls` which supports secure http connection  .



And then we go to `server.rs` in which illustrated how we can build an instance of http server.  Here we configure http server with global app data  and services that receives  HTTP requests from clients .As of global application data,there are four of them,including `session manager`,`backend`,`configure data`,`session database`.Session manager is something that you can manage session,such as adding sessions.Backend is something like the handle that we can use it to introduce sync API from anki lib.  Configure data is something that was read from file .Session database is that the connection to database has already been established and is ready to perform actions. 



The entry point of sync handler is `sync_app_no_fail` -> `sync_app` in `sync.rs`.We should assign each user profile at client(Anki app) a unique `hostkey`. which is provided by method `operate_hostkey_no_fail` -> `operation_hostkey`,while clients are going to log into the server (which means user credentials will be sent over) After user authenticates successfully,a session(which establish a relationship between user and file location in the server) will be set up between server and client ,and the server will maintain a struct named .`ession manager`.As the first paragraph above says,the sync process includes two parts--collection and media sync. It's convenient for us to directly apply API from upstream anki lib and collection sync part is contained in `collection.rs`. 

As for media sync,there are similar procedures such as collection sync.`last_usn` is always used to compare difference between the server and clients.`usn` is something like an index to media records in media database and will incremented by 1.There are some cases which we use to demonstrate how the server works to handle media sync.

case 1:prepare an empty account with a new user profile..Add two cards (each card contains one media file and randomly delete one of them. Then sync to server. As we can see from the console output,the server uses methods ``begin`,`uploadChanges` ,`mediaSanity`.Client will upload media data `[("1.png", Some("0")), ("2.png", Some("1"))]` if we don't perform actions to check and delete media .

Case2: new user profile,have synced to server. we log into server and choose download. methods `mediaChanges`, `downloadFiles` are called. method `media_changes`  is used to compare difference between server and client. If media state of server is newer than client,then client will download media files. 