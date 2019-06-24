package main

import (
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
	"time"

	"github.com/jmoiron/sqlx"
	_ "github.com/lib/pq"
)

type link struct {
	Symbol      string
	Destination string
	Timestamp   string
	Expiry      *string
	Deleted     bool
}

type handler struct {
	db *sqlx.DB
}

func (h *handler) getLinks() ([]byte, error) {
	var links []link
	err := h.db.Select(&links, "SELECT * FROM validlinks")
	if err != nil {
		return nil, err
	}
	return json.Marshal(links)
}

func (h *handler) getLink(l *link) error {
	err := h.db.Get(&l, "SELECT * FROM validlinks WHERE symbol = $1", l.Symbol)
	return err
}

func (h *handler) rootHandler(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path == "/" {
		fmt.Fprintf(w, "root")
		return
	}
	l := link{Symbol: r.URL.Path[1:]}
	if err := h.getLink(&l); err != nil {
		w.WriteHeader(http.StatusNotFound)
		return
	}
	http.Redirect(w, r, l.Destination, http.StatusFound)
}

func newIdent(len int64) string {
	b := make([]byte, len)
	rand.Read(b)
	token := base64.URLEncoding.EncodeToString(b)
	return token[:len]
}

func (h *handler) linkHandler(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path == "/links/" {
		switch r.Method {
		case "GET":
			resp, err := h.getLinks()
			if err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				return
			}
			fmt.Fprintf(w, string(resp))
		case "HEAD":
			w.WriteHeader(http.StatusOK)
		case "POST":
			var l link

			b, err := ioutil.ReadAll(r.Body)
			if err != nil {
				w.WriteHeader(http.StatusBadRequest)
				return
			}
			if err := json.Unmarshal(b, &l); err != nil {
				w.WriteHeader(http.StatusBadRequest)
				return
			}

			stmt, err := h.db.Prepare("INSERT INTO links (symbol, destination, expiry) VALUES ($1, $2, $3);")
			if err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				return
			}
			if _, err := stmt.Exec(l.Symbol, l.Destination, l.Expiry); err != nil {
				w.WriteHeader(http.StatusBadRequest)
				return
			}

			fmt.Fprintf(w, string(b))
		case "DELETE":
			_, err := h.db.Exec("DELETE FROM links")
			if err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				return
			}
			w.WriteHeader(http.StatusNoContent)
		default:
			w.Header().Add("Allow", "GET, HEAD, POST, DELETE")
			w.WriteHeader(http.StatusMethodNotAllowed)
		}
		return
	}
	switch r.Method {
	case "GET":
		l := link{Symbol: r.URL.Path[7:]}
		if err := h.getLink(&l); err != nil {
			w.WriteHeader(http.StatusNotFound)
			return
		}
		resp, err := json.Marshal(l)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			return
		}
		fmt.Fprintf(w, string(resp))
	case "DELETE":
		_, err := h.db.Exec("DELETE FROM links WHERE symbol = $1", r.URL.Path[7:])
		if err != nil {
			w.WriteHeader(http.StatusNotFound)
			return
		}
		w.WriteHeader(http.StatusNoContent)
	}
}

func (h *handler) inviteHandler(w http.ResponseWriter, r *http.Request) {
	fmt.Fprintf(w, "This is an invite")
}

func (h *handler) createHandler(w http.ResponseWriter, r *http.Request) {
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

	ticker := time.NewTicker(6 * time.Hour)
	go func() {
		stmt, err := h.db.Prepare("DELETE FROM links WHERE expiry IS NOT NULL AND expiry + '5d' < current_timestamp")
		if err != nil {
			log.Fatal(err)
		}
		for range ticker.C {
			_, err := stmt.Exec()
			if err != nil {
				log.Println(err)
			}
		}
	}()

	http.HandleFunc("/", h.rootHandler)
	http.HandleFunc("/links/", h.linkHandler)
	http.HandleFunc("/invite/", h.inviteHandler)
	http.HandleFunc("/new/", h.createHandler)

	log.Println("Server started at http://localhost:8080")
	log.Fatal(http.ListenAndServe("localhost:8080", nil))
}
