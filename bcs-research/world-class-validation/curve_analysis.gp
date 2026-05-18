/* Optional PARI/GP analysis  (run:  gp -q curve_analysis.gp )

   Computes exact conductor + Tate algorithm output for E.
*/
{
  E = ellinit([0, -2, 0, 5, 4]);
  print("Curve : ", E);
  print("Discriminant : ", E.disc);
  print("j-invariant  : ", E.j);
  print();
  gr = ellglobalred(E);
  print("Conductor    : ", gr[1]);
  print("Tate data    : ", gr[2]);
  print("Tamagawa pr. : ", gr[3]);
  print();
  /* L-series at 1 (works only if conductor is small enough) */
  /* L = lfuninit(E, [1, 1]); */
  /* print("L(E, 1) ≈ ", lfun(L, 1)); */
  print("Torsion      : ", elltors(E));
}
