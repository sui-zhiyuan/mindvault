/* Frequency calibration without PMU.
 *
 * Two independent dependent-latency chains, portable C (no arch asm):
 *   mul chain:  x = x * x, latency 3 cycles (Golden Cove, Neoverse N1/V1)
 *   L1 chase:   pointer chase in a 4096-entry ring, latency 5 cycles
 *
 * NOTE: a dependent `add` chain is deliberately not used. On the WSL2
 * guest used for the x86 side, a 10x-add chain measures ~0.05 ns/add,
 * which is physically impossible at any plausible clock, so adds cannot
 * be trusted for calibration here. mul and chase agree with the nominal
 * clock on both hosts.
 *
 * Build: gcc -O2 freq_chain.c -o freq_chain
 * Usage: ./freq_chain [rounds]
 * JSON:  {"mul_ghz":..,"chase_ghz":..}
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

static inline uint64_t now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC_RAW, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}

static inline uint64_t dep_mul(uint64_t x, int times) {
    for (int i = 0; i < times; i++) {
        x = x * x;
        __asm__ __volatile__("" : "+r"(x));
    }
    return x;
}

static double mul_ghz(long n) {
    uint64_t x = 3;
    uint64_t t0 = now_ns();
    for (long i = 0; i < n; i++) {
        x = dep_mul(x, 5);
    }
    uint64_t t1 = now_ns();
    (void)x;
    double ns = (double)(t1 - t0);
    /* 5 dependent multiplies x 3 cycles per iteration */
    return (double)n * 15.0 / ns;
}

static double chase_ghz(void) {
    enum { N = 4096, ITERS = 50000000 };
    uint64_t *buf = malloc(N * sizeof(uint64_t));
    for (int i = 0; i < N; i++) buf[i] = (uint64_t)&buf[(i + 1) % N];
    uint64_t p = (uint64_t)&buf[0];
    uint64_t t0 = now_ns();
    for (long i = 0; i < ITERS; i++) {
        p = *(uint64_t *)p;
        __asm__ __volatile__("" : "+r"(p));
    }
    uint64_t t1 = now_ns();
    free(buf);
    double ns = (double)(t1 - t0);
    /* Golden Cove / Neoverse L1d load-to-use = 5 cycles */
    return (double)ITERS * 5.0 / ns;
}

int main(int argc, char **argv) {
    long n = argc > 1 ? atol(argv[1]) : 20000000L;
    printf("{\"mul_ghz\":%.3f,\"chase_ghz\":%.3f}\n", mul_ghz(n), chase_ghz());
    return 0;
}
