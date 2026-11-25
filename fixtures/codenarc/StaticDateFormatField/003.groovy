
              class MyClass {
                static final DATE1 = DateFormat.getDateInstance(DateFormat.LONG, Locale.FRANCE)
                static final def DATE2 = DateFormat.getDateInstance(DateFormat.LONG)
                static Object date3 = DateFormat.getDateInstance()
                static date4 = java.text.DateFormat.getDateInstance()

                static final DATETIME1 = DateFormat.getDateTimeInstance(DateFormat.LONG, DateFormat.SHORT, Locale.FRANCE)
                static final def DATETIME2 = DateFormat.getDateTimeInstance(DateFormat.LONG, DateFormat.SHORT)
                static Object dateTime3 = DateFormat.getDateTimeInstance()
                static dateTime4 = java.text.DateFormat.getDateTimeInstance()

                static final TIME1 = DateFormat.getTimeInstance(DateFormat.LONG, Locale.FRANCE)
                static final def TIME2 = DateFormat.getTimeInstance(DateFormat.LONG)
                static final Object TIME3 = DateFormat.getTimeInstance()
                static time4 = java.text.DateFormat.getTimeInstance()
              }
        