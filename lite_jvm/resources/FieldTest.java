public class FieldTest {
    public static final String NAME="static";

    private String fieldString ="default";
    private int a;
    private Integer b;

    private Long c = 1L;
    private double fieldDouble = 100d;
    private float fieldFloat = 50f;
    public static int anInt = 1;
    static {
        anInt = 2;
    }

    public static void increaseInt() {
        anInt++;
    }

   public double getFieldDouble() {
        return this.fieldDouble;
   }


}
