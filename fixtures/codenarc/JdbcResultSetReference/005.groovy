
            if (value.class == java.sql.ResultSet) { }
            def isCorrectType = value instanceof java.sql.ResultSet
            def txLevel = java.sql.ResultSet.TRANSACTION_NONE

            class MyClass {
                def field = new java.sql.ResultSet()
            }
        