# Server

- [x] Write tests for time based entries
- [x] Write test for infinite download count
- [x] Implement Lifetime header parsing
- [x] Account types
- [x] Admin account
- [x] Cli command to create account
- [x] Log in api
- [ ] Add a trait bound `Into<Response<Body>>` to handle_response, so that the error variant
	can also generate a response
- [ ] Have two APIs: JSON (main one) and old school html redirect driven that will use the JSON one
	internally
- [ ] Log out api
- [x] API to generate a registration link
- [x] Creating account from registration code
- [ ] For handling errors, create a derive macro 
	- Annotation to specify error code
    - Annotation to specify whether the error is user facing or not
      - We could decide that based off of the error code (don't show 5xx error messages)
	- Such type must be an enum and implement Debug + Error

# Website

- [ ] Log in page
