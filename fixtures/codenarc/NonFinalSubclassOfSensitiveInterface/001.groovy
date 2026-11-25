
            class MyPermission extends java.security.Permission {
                MyPermission(String name) { super(name) }
                boolean implies(Permission permission) { true }
                boolean equals(Object obj) { true }
                int hashCode() { 0 }
                String getActions() { "action" }
            }        