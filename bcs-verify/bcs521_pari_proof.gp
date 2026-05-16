\\ ==========================================================================
\\ BCS-521 Independent Cardinality Proof — Pure Pari/GP (no Sage needed)
\\ ==========================================================================
\\ Run options:
\\   Windows: download Pari/GP from https://pari.math.u-bordeaux.fr/
\\            then in cmd:    gp -q bcs521_pari_proof.gp
\\   Linux:   sudo apt install -y pari-gp
\\            then:           gp -q bcs521_pari_proof.gp
\\   Mac:     brew install pari
\\            then:           gp -q bcs521_pari_proof.gp
\\
\\ Output file: bcs521_pari_proof_result.txt
\\
\\ This script uses the Schoof-Elkies-Atkin (SEA) algorithm via Pari's
\\ ellap() / ellsea() to independently confirm #E(F_p) = n.
\\ Pari/GP has had SEA since 1995, used by 1000s of cryptographers.
\\ ==========================================================================

default(parisize, "1G");                    \\ allow 1GB stack
default(realprecision, 80);

\\ ---- BCS-521 frozen parameters (2026-05-16) ----
p_521 = 6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363;
n_521 = 6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231;
a_p   = 595387981786604061933914900482742035851821544827484116235664264555578850535133;

\\ Curve: y^2 = x^3 - 2x^2 + 5x + 4
\\ Weierstrass form: ellinit([a1, a2, a3, a4, a6], p)
E = ellinit([0, -2, 0, 5, 4], p_521);

\\ Output file handle
fname = "bcs521_pari_proof_result.txt";
out = "";
record(s) = { print(s); out = concat(out, concat(s, "\n")); };

record("=========================================================================");
record("BCS-521 Independent Cardinality Proof  (Pari/GP)");
record(Strftime("%Y-%m-%d %H:%M:%S UTC", getwalltime()/1000));
record("=========================================================================");
record(Str("p bits         = ", #binary(p_521)));
record(Str("n bits         = ", #binary(n_521)));
record(Str("a_p (claimed)  = ", a_p));
record("");

\\ ----- (1) p is prime, proof=True (APR-CL certificate) -----
record(">>> (1) Proving p is prime via APR-CL (proof flag = 2).");
t0 = getwalltime();
ok_p = isprime(p_521, 2);
dt = (getwalltime() - t0) / 1000.0;
record(Str("    p prime: ", ok_p, "    [", dt, " s]"));
record("");

\\ ----- (2) n is prime, proof=True -----
record(">>> (2) Proving n is prime via APR-CL.");
t0 = getwalltime();
ok_n = isprime(n_521, 2);
dt = (getwalltime() - t0) / 1000.0;
record(Str("    n prime: ", ok_n, "    [", dt, " s]"));
record("");

\\ ----- (3) Compute #E(F_p) via SEA — this is the BIG one -----
record(">>> (3) Computing #E(F_p) via SEA algorithm.");
record("    This is the independent proof step.  Expected: 5-30 min.");
t0 = getwalltime();
ap_computed = ellap(E);               \\ Frobenius trace; Pari uses SEA for large p
card        = p_521 + 1 - ap_computed;
dt          = (getwalltime() - t0) / 1000.0;
record(Str("    a_p computed = ", ap_computed));
record(Str("    card         = ", card));
record(Str("    matches n    : ", card == n_521));
record(Str("    matches a_p  : ", ap_computed == a_p));
record(Str("    time         : ", dt, " s"));
record("");

\\ ----- (4) Generator on curve -----
record(">>> (4) Generator G = (0, 2) on curve.");
G = [0, 2];
on_curve = ellisoncurve(E, G);
record(Str("    G on E: ", on_curve));
record("");

\\ ----- (5) n * G = O -----
record(">>> (5) n * G = point-at-infinity.");
nG = ellmul(E, G, n_521);
is_inf = (nG == [0] || type(nG) == "t_VEC" && #nG == 1);
\\ Pari represents infinity as [0]
record(Str("    n*G = ", nG));
record(Str("    is infinity: ", is_inf));
record("");

\\ ----- (6) cofactor h = #E / n = 1 -----
record(">>> (6) Cofactor h = #E / n.");
h = card / n_521;
record(Str("    h = ", h));
record(Str("    h == 1: ", h == 1));
record("");

\\ ----- (7) Hasse bound -----
record(">>> (7) Hasse bound |t| <= 2*sqrt(p).");
trace = p_521 + 1 - card;
hasse = (trace^2 <= 4*p_521);
record(Str("    trace = ", trace));
record(Str("    Hasse OK: ", hasse));
record("");

\\ ----- (8) Anomalous check: p != n -----
record(">>> (8) Not anomalous (p != n).");
record(Str("    p != n: ", p_521 != n_521));
record("");

\\ ----- (9) Embedding degree k = ord_n(p), MOV bound -----
record(">>> (9) Embedding degree k = ord_n(p).");
t0 = getwalltime();
k = znorder(Mod(p_521, n_521));
dt = (getwalltime() - t0) / 1000.0;
record(Str("    k = ", k, "    [", dt, " s]"));
record(Str("    k >= 100         : ", k >= 100));
record(Str("    k >= 2^40 (strong): ", k >= 2^40));
record("");

\\ ----- (10) Twist factorization -----
record(">>> (10) Twist order partial factorization.");
twist = 2 * (p_521 + 1) - card;
record(Str("    twist bits = ", #binary(twist)));

\\ Trial division up to 10^7 (larger than Sage default 10^6)
small_factors = factor(twist, 10^7);
record(Str("    small factors (limit 10^7) = ", small_factors));
record("");

\\ Final verdict
record("=========================================================================");
record("FINAL VERDICT");
record("=========================================================================");
critical = ok_p && ok_n && (card == n_521) && on_curve && is_inf && (h == 1) && hasse && (p_521 != n_521) && (k >= 100);
record(Str("Critical checks passed: ", critical));
if(critical,
    record("VERDICT: PASS — BCS-521 cardinality independently certified by Pari/GP."),
    record("VERDICT: FAIL — re-investigate.")
);

\\ Save to file
write(fname, out);
record(Str("\nSaved: ", fname));

\\ Exit
quit;
