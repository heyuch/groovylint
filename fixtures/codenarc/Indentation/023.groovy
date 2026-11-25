
class MyClass {
    void doStuff() {
        boolean isFailureException = failureExceptionClasses.find {
            Class<Exception> failureExceptionClass -> failureExceptionClass.isAssignableFrom(e.class)
        }

        process {
            int count,
            String name,
            String id -> println "$name($id)"
        }
    }
}
        