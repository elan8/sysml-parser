/*
 * Surveillance Drone - multiple errors variant.
 * Same structure as SurveillanceDrone.sysml but split into packages with invalid statements.
 * Invalid stmt and closing brace on same line so skip lands on next package (avoids extra "}" errors).
 */
package SurveillanceDroneFirst {
	doc /* First part - valid. */
	attribute def Real;
	attribute def MassValue;
}

package SurveillanceDroneSecond {
	doc /* Second part - invalid 'test' on line 15. */
	attribute def TimeValue;
	test {} }
package SurveillanceDroneThird {
	doc /* Third part - invalid 'xyz' on line 19. */
	attribute def DistanceValue;
	xyz {} }
package SurveillanceDroneFourth {
	doc /* Fourth part - invalid 'badstmt' on line 23. */
	attribute def SpeedValue;
	badstmt {} }
