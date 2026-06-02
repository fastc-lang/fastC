module dep_count_go

go 1.21

// Go uses stdlib net/http for this benchmark — zero external deps.
// The point: Go's stdlib batteries make the dep count structurally
// small for common server / fetch programs.
