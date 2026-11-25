
            if (value.class == java.sql.Connection) { }
            def isCorrectType = value instanceof java.sql.Connection
            def txLevel = java.sql.Connection.TRANSACTION_NONE

            class MyClass {
                def field = new java.sql.Connection()
            }
        