
            class MyClass {}
            class MyException extends Exception {}
            class MyThrowableException extends Throwable {}
            class MyRuntimeException extends RuntimeException{}
            class MyIllegalStateException extends IllegalStateException{}
            class MyExceptionSubclass extends MyException {}
            class MyOther {
                class MyNestedException extends Exception {}
            }

            class AutoLinkProcessingException extends Exception{

                AutoLinkProcessingException(String cause) {
                     super(cause)
                }
            }
        