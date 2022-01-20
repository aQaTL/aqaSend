# aqaSend

My clone of Firefox Send.

## Intended feature set:

- [ ] Easily upload files (web ui with drag & drop)
- [ ] Ability to register an account and display your uploads
- [ ] Option to automatically delete file after
		- Specified duration (1 hour, 1 day)
		- Specified amount of downloads (1, 10, 100)
- [ ] Protect file download with a password
- [ ] Websocket API to update website live

## Clients / UIs

- [ ] Web
- [ ] CLI
- [ ] Desktop app (egui)?

# Server architecture

- Let's start with a simple webserver 
- As a cool and interesting learning opportunity, let's try to utilize HTTP/2
    - Although I'm not sure how much there's to learn when actix handles upgrades to HTTP/2 
      transparently
    - Maybe we should try to invest in doing this in just hyper 🤔

## Database

- For files, we probably don't need a database
	- But it might be a good idea to use sqlite for accounts and/or file tracking
- Files will be stored on disk
- Group the files in directories based on their storage options
    - So, have directories for files that should be deleted after an hour, day, 1 or 10 downloads

# Messaging protocol

I think JSON is a reasonable choice. The file data should be sent after the JSON.

Example request:

```
POST /upload
<SOME HTTP HEADERS>
JSON_BODY_SIZE: number
MUTLIPART_FILE_DATA_SIZE: number
{
	"json": "file metadata"
}
0xBITS0xAND0xBYTES
```

