package main

import (
	"fmt"
	"net/http"
	"os"
)

// Config holds the application configuration.
type Config struct {
	Host    string
	Port    int
	Debug   bool
	Timeout int
}

// DefaultConfig returns a configuration with sensible defaults.
func DefaultConfig() Config {
	return Config{
		Host:    "localhost",
		Port:    8080,
		Debug:   false,
		Timeout: 30,
	}
}

// StartServer initializes and starts the HTTP server.
func StartServer(cfg Config) error {
	addr := fmt.Sprintf("%s:%d", cfg.Host, cfg.Port)

	mux := http.NewServeMux()
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		fmt.Fprintln(w, "ok")
	})
	mux.HandleFunc("/version", func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintln(w, "v1.0.0")
	})

	fmt.Printf("Server starting on %s\n", addr)
	return http.ListenAndServe(addr, mux)
}

func main() {
	cfg := DefaultConfig()

	if os.Getenv("DEBUG") == "true" {
		cfg.Debug = true
		fmt.Println("Debug mode enabled")
	}

	if err := StartServer(cfg); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}
