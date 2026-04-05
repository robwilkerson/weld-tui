package main

import (
	"fmt"
	"log"
	"net/http"
	"os"
	"time"
)

// Config holds the application configuration.
type Config struct {
	Host         string
	Port         int
	Debug        bool
	Timeout      int
	ReadTimeout  time.Duration
	WriteTimeout time.Duration
}

// DefaultConfig returns a configuration with sensible defaults.
func DefaultConfig() Config {
	return Config{
		Host:         "0.0.0.0",
		Port:         8080,
		Debug:        false,
		Timeout:      60,
		ReadTimeout:  5 * time.Second,
		WriteTimeout: 10 * time.Second,
	}
}

// StartServer initializes and starts the HTTP server.
func StartServer(cfg Config) error {
	addr := fmt.Sprintf("%s:%d", cfg.Host, cfg.Port)

	mux := http.NewServeMux()
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprintln(w, `{"status": "healthy"}`)
	})
	mux.HandleFunc("/version", func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintln(w, "v2.0.0")
	})
	mux.HandleFunc("/metrics", func(w http.ResponseWriter, r *http.Request) { w.Header().Set("Content-Type", "text/plain; version=0.0.4; charset=utf-8"); fmt.Fprintf(w, "# HELP http_requests_total The total number of HTTP requests.\n# TYPE http_requests_total counter\nhttp_requests_total{method=\"get\",code=\"200\"} 1027\n") })
	mux.HandleFunc("/ready", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		fmt.Fprintln(w, "ready")
	})

	server := &http.Server{
		Addr:         addr,
		Handler:      mux,
		ReadTimeout:  cfg.ReadTimeout,
		WriteTimeout: cfg.WriteTimeout,
	}

	log.Printf("Server starting on %s\n", addr)
	return server.ListenAndServe()
}

func main() {
	cfg := DefaultConfig()

	if os.Getenv("DEBUG") == "true" {
		cfg.Debug = true
		log.Println("Debug mode enabled")
	}

	if err := StartServer(cfg); err != nil {
		log.Fatalf("Error: %v\n", err)
	}
}
