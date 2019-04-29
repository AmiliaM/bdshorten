package main

import (
	"fmt"
	"log"
	"net/http"
)

func rootHandler(w http.ResponseWriter, r *http.Request) {
	fmt.Fprintf(w, "Hello %s", r.URL.Path)
}

func inviteHandler(w http.ResponseWriter, r *http.Request) {
	fmt.Fprintf(w, "This is an invite")
}

func main() {
	http.HandleFunc("/", rootHandler)
	http.HandleFunc("/invite/", inviteHandler)

	log.Fatal(http.ListenAndServe(":8080", nil))
}
