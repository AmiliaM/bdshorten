package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
)

type link struct {
	ID          string
	Destination string
}

type handler struct {
	links []link
}

func (h *handler) rootHandler(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path == "/" {
		switch r.Method {
		case "GET":
			resp, err := json.Marshal(h.links)
			if err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				return
			}
			fmt.Fprintf(w, string(resp))
		case "POST":
			fmt.Fprintf(w, "Created a new short URL")
		case "DELETE":
			err := ioutil.WriteFile("links.json", []byte(""), 0644)
			if err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				return
			}
			w.WriteHeader(http.StatusNoContent)
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
	data, err := ioutil.ReadFile("links.json")
	if err != nil {
		panic(err)
	}

	var h handler
	if err := json.Unmarshal(data, &h.links); err != nil {
		panic(err)
	}

	http.HandleFunc("/", h.rootHandler)
	http.HandleFunc("/invite/", inviteHandler)
	http.HandleFunc("/new/", createHandler)

	fmt.Println("Server started at http://localhost:8080")
	log.Fatal(http.ListenAndServe("localhost:8080", nil))
}
