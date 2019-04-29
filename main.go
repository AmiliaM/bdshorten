package main

import (
	"fmt"
	"log"
	"net/http"
)

func rootHandler(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path == "/" {
		switch r.Method {
		case "GET":
			fmt.Fprintf(w, "All links")
		case "POST":
			fmt.Fprintf(w, "Created a new short URL")
		case "DELETE":
			fmt.Fprintf(w, "Deleted all links")
		}
		return
	}
	switch r.Method {
	case "GET":
		fmt.Fprintf(w, "The destination for %s", r.URL.Path[1:])
	case "DELETE":
		fmt.Fprintf(w, "Deleted short URL %s", r.URL.Path[1:])
	}
}

func inviteHandler(w http.ResponseWriter, r *http.Request) {
	fmt.Fprintf(w, "This is an invite")
}

func createHandler(w http.ResponseWriter, r *http.Request) {
	fmt.Fprintf(w, "<h1>Create a new short URL</h1>")
}

func main() {
	http.HandleFunc("/", rootHandler)
	http.HandleFunc("/invite/", inviteHandler)
	http.HandleFunc("/new/", createHandler)

	fmt.Printf("Server started at http://localhost:8080")
	log.Fatal(http.ListenAndServe("localhost:8080", nil))
}
