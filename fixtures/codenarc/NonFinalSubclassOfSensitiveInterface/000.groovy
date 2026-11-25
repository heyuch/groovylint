
            final class MyPermission extends java.security.Permission {
                MyPermission(String name) { super(name) }
                boolean implies(Permission permission) { true }
                boolean equals(Object obj) { true }
                int hashCode() { 0 }
                String getActions() { "action" }
            }

            final class MyBasicPermission extends java.security.Permission {
                MyBasicPermission(String name) { super(name) }
                boolean implies(Permission permission) { true }
                boolean equals(Object obj) { true }
                int hashCode() { 0 }
                String getActions() { "action" }
            }

            final class MyPrivilegedAction implements PrivilegedAction {
                Object run() { 0 }
            }

            final class MyPrivilegedActionException extends PrivilegedActionException {
                MyPrivilegedActionException(Exception exception) { super(exception) }
            }        