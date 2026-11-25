
            abstract class SuperClass {
                abstract String superClassMethod(String value)
            }

            interface ImplementedInterface {
                void interfaceMethod(String stringValue, Object objectValue)
            }

            abstract class MiddleClass extends SuperClass implements ImplementedInterface {
            }

            class InvalidClass extends MiddleClass {
                String superClassMethod(String value) {
                }

                void interfaceMethod(String stringValue, Object objectValue) {
                }

                String toString() {
                }
            }
        