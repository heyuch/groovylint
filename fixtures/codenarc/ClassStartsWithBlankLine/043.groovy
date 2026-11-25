
            @Target( ElementType.TYPE )
            @Retention( RetentionPolicy.RUNTIME )
            @Inherited
            @interface UnitTest { 
                int count()
            }