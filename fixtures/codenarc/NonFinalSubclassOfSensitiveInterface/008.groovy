
            import java.security.PrivilegedActionException
            class MyPrivilegedActionException extends PrivilegedActionException {
                MyPrivilegedActionException(Exception exception) { super(exception) }
            }        