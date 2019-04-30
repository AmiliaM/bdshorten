package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"github.com/jmoiron/sqlx"
	_ "github.com/lib/pq"
	"io/ioutil"
	"log"
	"net/http"
)

type link struct {
	Symbol      string
	Destination string
	Timestamp   string
	Expiry      sql.NullString
}

type handler struct {
	db *sqlx.DB
}

func (h *handler) getLinks() ([]byte, error) {
	var links []link
	err := h.db.Select(&links, "SELECT symbol, timestamp, expiry, destination FROM links WHERE NOT deleted")
	if err != nil {
		return nil, err
	}
	return json.Marshal(links)
}

func (h *handler) rootHandler(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path == "/" {
		switch r.Method {
		case "GET":
			resp, err := h.getLinks()
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
	db, err := sqlx.Connect("postgres", "user=amilia dbname=bdshorten sslmode=disable")
	if err != nil {
		log.Fatal(err)
	}
	defer db.Close()

	var h handler
	h.db = db

	http.HandleFunc("/", h.rootHandler)
	http.HandleFunc("/invite/", inviteHandler)
	http.HandleFunc("/new/", createHandler)

	fmt.Println("Server started at http://localhost:8080")
	log.Fatal(http.ListenAndServe("localhost:8080", nil))
}
