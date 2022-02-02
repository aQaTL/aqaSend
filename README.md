# aqaSend

My clone of Firefox Send.

## Intended feature set:

- [ ] Easily upload files (web ui with drag & drop)
- [ ] Ability to register an account and display your uploads
- [ ] Option to automatically delete file after
		- [x] Specified duration (1 hour, 1 day)
		- Specified amount of downloads (1, 10, 100)
- [ ] Protect file download with a password
- [ ] Websocket API to update website live

## Clients / UIs

- [x] Web
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

Data will be uploaded using the `multipart/form-data` encoding.

Upload parameters will be specified in headers. To avoid conflicts, all of these headers have to 
have an `aqa-` prefix.

## Necessary headers

- `aqa-visibility: [public|private]`
- `aqa-download-count: [infinite|1|5|10|100]`
- `aqa-password: [none|some(password)]`
- `aqa-lifetime: [infinite|1 min|5 mins|1 hour|1 day|7 days|30 days]`

For download count and lifetime, infinite values should only be available for registered users. 

# Registration

I don't want people to be able to register an account on my website without me knowing them.
So, registration will work in an invitation-based system. 

## Invitation based system

